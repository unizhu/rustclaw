//! Custom HTTP client wrapper for MCP Streamable HTTP transport.
//!
//! Wraps `reqwest::Client` to handle server compatibility issues where
//! `200 OK` with empty body is returned instead of `202 Accepted`.

use std::sync::Arc;

use futures::stream::BoxStream;
use rmcp::{
    model::{ClientJsonRpcMessage, ServerJsonRpcMessage},
    transport::streamable_http_client::{
        StreamableHttpClient, StreamableHttpError, StreamableHttpPostResponse,
    },
};
use sse_stream::{Error as SseError, Sse, SseStream};
use tracing::debug;

use futures::StreamExt;

/// Header names used by MCP Streamable HTTP protocol
const HEADER_SESSION_ID: &str = "mcp-session-id";
const HEADER_LAST_EVENT_ID: &str = "last-event-id";
const EVENT_STREAM_MIME_TYPE: &str = "text/event-stream";
const JSON_MIME_TYPE: &str = "application/json";

/// Custom HTTP client that wraps `reqwest::Client` with compatibility fixes.
///
/// Some MCP servers (e.g., BigModel/ZhipuAI) return `200 OK` with an empty body
/// for notification responses, instead of the `202 Accepted` that the rmcp SDK expects.
/// This wrapper treats `200` with an empty/missing content-type body as "Accepted".
#[derive(Clone, Debug, Default)]
pub struct CompatibleHttpClient {
    inner: reqwest::Client,
}

impl StreamableHttpClient for CompatibleHttpClient {
    type Error = reqwest::Error;

    async fn get_stream(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        last_event_id: Option<String>,
        auth_token: Option<String>,
    ) -> Result<BoxStream<'static, Result<Sse, SseError>>, StreamableHttpError<Self::Error>> {
        let mut request_builder = self
            .inner
            .get(uri.as_ref())
            .header(
                reqwest::header::ACCEPT,
                [EVENT_STREAM_MIME_TYPE, JSON_MIME_TYPE].join(", "),
            )
            .header(HEADER_SESSION_ID, session_id.as_ref());
        if let Some(last_event_id) = last_event_id {
            request_builder = request_builder.header(HEADER_LAST_EVENT_ID, last_event_id);
        }
        if let Some(auth_header) = auth_token {
            request_builder = request_builder.bearer_auth(auth_header);
        }
        let response = request_builder
            .send()
            .await
            .map_err(StreamableHttpError::Client)?;
        if response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED {
            return Err(StreamableHttpError::ServerDoesNotSupportSse);
        }
        let response = response
            .error_for_status()
            .map_err(StreamableHttpError::Client)?;
        match response.headers().get(reqwest::header::CONTENT_TYPE) {
            Some(ct) => {
                if !ct.as_bytes().starts_with(EVENT_STREAM_MIME_TYPE.as_bytes())
                    && !ct.as_bytes().starts_with(JSON_MIME_TYPE.as_bytes())
                {
                    return Err(StreamableHttpError::UnexpectedContentType(Some(
                        String::from_utf8_lossy(ct.as_bytes()).to_string(),
                    )));
                }
            }
            None => {
                return Err(StreamableHttpError::UnexpectedContentType(None));
            }
        }
        let event_stream = SseStream::from_byte_stream(response.bytes_stream()).boxed();
        Ok(event_stream)
    }

    async fn delete_session(
        &self,
        uri: Arc<str>,
        session: Arc<str>,
        auth_token: Option<String>,
    ) -> Result<(), StreamableHttpError<Self::Error>> {
        let mut request_builder = self.inner.delete(uri.as_ref());
        if let Some(auth_header) = auth_token {
            request_builder = request_builder.bearer_auth(auth_header);
        }
        let response = request_builder
            .header(HEADER_SESSION_ID, session.as_ref())
            .send()
            .await
            .map_err(StreamableHttpError::Client)?;

        if response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED {
            debug!("this server doesn't support deleting session");
            return Ok(());
        }
        let _response = response
            .error_for_status()
            .map_err(StreamableHttpError::Client)?;
        Ok(())
    }

    async fn post_message(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_token: Option<String>,
    ) -> Result<StreamableHttpPostResponse, StreamableHttpError<Self::Error>> {
        let mut request = self.inner.post(uri.as_ref()).header(
            reqwest::header::ACCEPT,
            [EVENT_STREAM_MIME_TYPE, JSON_MIME_TYPE].join(", "),
        );
        if let Some(ref auth_header) = auth_token {
            debug!(auth_token_preview = %format!("{}...", &auth_header[..auth_header.len().min(10)]), "Setting bearer auth");
            request = request.bearer_auth(auth_header);
        }
        if let Some(session_id) = session_id {
            request = request.header(HEADER_SESSION_ID, session_id.as_ref());
        }
        let response = request
            .json(&message)
            .send()
            .await
            .map_err(StreamableHttpError::Client)?;

        let status = response.status();
        let content_type_header = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .map(|ct| String::from_utf8_lossy(ct.as_bytes()).to_string());
        let content_len_header = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .map(|cl| String::from_utf8_lossy(cl.as_bytes()).to_string());

        debug!(
            %status,
            content_type = ?content_type_header,
            content_length = ?content_len_header,
            uri = %uri,
            "MCP HTTP response received"
        );

        if status == reqwest::StatusCode::UNAUTHORIZED {
            if let Some(header) = response.headers().get(http::header::WWW_AUTHENTICATE) {
                let header = header
                    .to_str()
                    .map_err(|_| {
                        StreamableHttpError::UnexpectedServerResponse(std::borrow::Cow::from(
                            "invalid www-authenticate header value",
                        ))
                    })?
                    .to_string();
                return Err(StreamableHttpError::AuthRequired(
                    rmcp::transport::streamable_http_client::AuthRequiredError {
                        www_authenticate_header: header,
                    },
                ));
            }
        }

        // Standard 202/204 → Accepted
        if matches!(
            status,
            reqwest::StatusCode::ACCEPTED | reqwest::StatusCode::NO_CONTENT
        ) {
            return Ok(StreamableHttpPostResponse::Accepted);
        }

        let session_id = response
            .headers()
            .get(HEADER_SESSION_ID)
            .and_then(|v| v.to_str().ok())
            .map(std::string::ToString::to_string);

        // Compatibility fix: 200 OK with no content-type → Accepted
        if status == reqwest::StatusCode::OK && content_type_header.is_none() {
            debug!("200 OK with no content-type, treating as Accepted");
            return Ok(StreamableHttpPostResponse::Accepted);
        }

        match &content_type_header {
            Some(ct) if ct.starts_with(EVENT_STREAM_MIME_TYPE) => {
                debug!("Routing to SSE path");
                let event_stream = SseStream::from_byte_stream(response.bytes_stream()).boxed();
                Ok(StreamableHttpPostResponse::Sse(event_stream, session_id))
            }
            Some(ct) if ct.starts_with(JSON_MIME_TYPE) => {
                debug!("Routing to JSON path");
                // Use text() + from_str() instead of response.json() to avoid
                // reqwest wrapping serde errors as Decode (hard to distinguish)
                let body = response.text().await.map_err(StreamableHttpError::Client)?;
                debug!(body_len = body.len(), body = %body, "JSON response body received");
                let message: ServerJsonRpcMessage =
                    serde_json::from_str(&body).map_err(StreamableHttpError::Deserialize)?;
                Ok(StreamableHttpPostResponse::Json(message, session_id))
            }
            _ => {
                let body = response.text().await.unwrap_or_default();
                tracing::error!(
                    content_type = ?content_type_header,
                    body_preview = %body.chars().take(200).collect::<String>(),
                    "unexpected content type"
                );
                Err(StreamableHttpError::UnexpectedContentType(
                    content_type_header,
                ))
            }
        }
    }
}
