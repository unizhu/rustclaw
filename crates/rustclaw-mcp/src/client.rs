//! MCP client wrapper around rmcp SDK
//!
//! Provides [`MCPClient`] for connecting to MCP servers via stdio or Streamable HTTP
//! transports, discovering available tools, and executing tool calls.

use crate::config::{MCPServerConfig, TransportType};
use crate::error::{MCPError, Result};
use crate::http_client::CompatibleHttpClient;
use rmcp::model::{
    CallToolRequestParams, ClientCapabilities, ClientInfo, Implementation, ProtocolVersion,
};
use rmcp::service::{Peer, RoleClient, RunningService};
use rmcp::transport::streamable_http_client::StreamableHttpClientTransport;
use rmcp::ServiceExt;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// MCP tool definition discovered from a server
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: Option<String>,
    /// Input schema (JSON Schema)
    pub input_schema: Value,
}

/// Handle to a running MCP server connection
///
/// Wraps the rmcp `Peer` which allows sending requests to the server.
/// The `Peer` is `Clone + Send + Sync` so it can be shared safely.
pub struct MCPClient {
    /// Server name
    pub name: String,
    /// Available tools discovered from the server
    pub tools: Vec<ToolDefinition>,
    /// Negotiated protocol version
    pub protocol_version: String,
    /// Peer handle for sending requests to the server
    peer: Peer<RoleClient>,
    /// Keep the running service alive — dropping it shuts down the connection
    _service: Arc<RwLock<Option<Box<dyn std::any::Any + Send + Sync>>>>,
}

/// Build the `ClientInfo` advertised during MCP initialization
fn client_info() -> ClientInfo {
    ClientInfo {
        protocol_version: ProtocolVersion::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "rustclaw".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            title: None,
            description: None,
            icons: None,
            website_url: None,
        },
        meta: None,
    }
}

impl MCPClient {
    /// Start an MCP server and connect to it
    ///
    /// Auto-detects transport type from the server configuration and
    /// establishes a connection with the given timeout.
    ///
    /// # Errors
    /// Returns an error if the server fails to start or times out
    pub async fn start(name: String, config: &MCPServerConfig, timeout: Duration) -> Result<Self> {
        info!("Starting MCP server '{}' with timeout {:?}", name, timeout);

        let transport_type = config.detect_transport();

        let result = tokio::time::timeout(timeout, async {
            match transport_type {
                TransportType::Stdio { program, args, env } => {
                    Self::start_stdio(&name, &program, &args, &env).await
                }
                TransportType::HTTP(url, headers) => {
                    // Case-insensitive lookup for Authorization header
                    let auth_header = headers
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
                        .map(|(_, v)| v.clone());

                    if auth_header.is_none() {
                        tracing::warn!(
                            ?headers,
                            "No Authorization header found for HTTP transport! Keys: {:?}",
                            headers.keys()
                        );
                    }
                    Self::start_http(&name, &url, auth_header).await
                }
            }
        })
        .await
        .map_err(|_| MCPError::StartupTimeout {
            server: name.clone(),
            timeout,
        })?;

        result
    }

    /// Start an MCP server via stdio (child process) transport
    async fn start_stdio(
        name: &str,
        program: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<Self> {
        debug!(
            "Starting stdio transport for '{}': {} {:?} env={:?}",
            name, program, args, env
        );

        // Build tokio Command for the child process
        let mut cmd = tokio::process::Command::new(program);
        cmd.args(args);

        // Set custom environment variables
        for (key, value) in env {
            cmd.env(key, value);
            // Also set uppercase version in case config loader lowercased it
            // (e.g. Z_AI_API_KEY becoming z_ai_api_key)
            let upper = key.to_uppercase();
            if upper != *key {
                cmd.env(upper, value);
            }
        }

        // Pipe stdin/stdout for MCP communication
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        // Create transport from child process (takes ownership of cmd)
        let transport =
            rmcp::transport::TokioChildProcess::new(cmd).map_err(|e| MCPError::StartupFailed {
                server: name.into(),
                reason: format!("Failed to spawn '{program}': {e}"),
            })?;

        // Connect and initialize MCP protocol
        let service: RunningService<RoleClient, _> = client_info()
            .serve(transport)
            .await
            .map_err(|e| MCPError::Sdk(format!("Failed to initialize MCP for '{name}': {e}")))?;

        let peer = service.peer().clone();

        // Discover tools from the server
        let tools = Self::discover_tools(&peer, name).await?;

        let protocol_version = "2024-11-05".to_string();

        info!(
            "MCP server '{}' connected via stdio ({} tools)",
            name,
            tools.len()
        );

        Ok(Self {
            name: name.into(),
            tools,
            protocol_version,
            peer,
            _service: Arc::new(RwLock::new(Some(Box::new(service)))),
        })
    }

    /// Start an MCP server via Streamable HTTP transport
    async fn start_http(name: &str, url: &str, auth_header: Option<String>) -> Result<Self> {
        debug!("Starting HTTP transport for '{}': {}", name, url);

        // Build transport config
        let mut config =
            rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(
                url,
            );

        // rmcp's reqwest impl uses `bearer_auth()` which adds "Bearer " prefix automatically,
        // so we strip the "Bearer " prefix from our config to avoid "Bearer Bearer xxx".
        if let Some(auth) = &auth_header {
            let token = auth.strip_prefix("Bearer ").unwrap_or(auth);
            config = config.auth_header(token.to_string());
        }

        let transport =
            StreamableHttpClientTransport::with_client(CompatibleHttpClient::default(), config);

        // Connect and initialize MCP protocol
        let service: RunningService<RoleClient, _> = client_info()
            .serve(transport)
            .await
            .map_err(|e| MCPError::Sdk(format!("Failed to initialize MCP for '{name}': {e}")))?;

        let peer = service.peer().clone();

        // Discover tools from the server
        let tools = Self::discover_tools(&peer, name).await?;

        let protocol_version = "2025-03-26".to_string();

        info!(
            "MCP server '{}' connected via HTTP ({} tools)",
            name,
            tools.len()
        );

        Ok(Self {
            name: name.into(),
            tools,
            protocol_version,
            peer,
            _service: Arc::new(RwLock::new(Some(Box::new(service)))),
        })
    }

    /// Discover available tools from a connected MCP server
    async fn discover_tools(peer: &Peer<RoleClient>, name: &str) -> Result<Vec<ToolDefinition>> {
        let list_result = peer
            .list_tools(None)
            .await
            .map_err(|e| MCPError::Sdk(format!("Failed to list tools from '{name}': {e}")))?;

        let tools: Vec<ToolDefinition> = list_result
            .tools
            .into_iter()
            .map(|t| {
                debug!("  Tool '{}': {:?}", t.name, t.description);
                ToolDefinition {
                    name: t.name.to_string(),
                    description: t.description.map(|d| d.to_string()),
                    input_schema: serde_json::to_value(&t.input_schema).unwrap_or_default(),
                }
            })
            .collect();

        info!("Discovered {} tools from '{}'", tools.len(), name);
        Ok(tools)
    }

    /// Call a tool on this MCP server
    ///
    /// # Errors
    /// Returns an error if the tool call fails
    pub async fn call_tool(&self, tool_name: &str, args: Value) -> Result<Value> {
        debug!("Calling tool '{}' on server '{}'", tool_name, self.name);

        let arguments = match args {
            Value::Object(map) => Some(map),
            Value::Null => None,
            other => {
                warn!(
                    "Tool '{}' called with non-object args, wrapping: {:?}",
                    tool_name, other
                );
                let mut map = serde_json::Map::new();
                map.insert("input".into(), other);
                Some(map)
            }
        };

        let result = self
            .peer
            .call_tool(CallToolRequestParams {
                name: String::from(tool_name).into(),
                arguments,
                meta: None,
                task: None,
            })
            .await
            .map_err(|e| MCPError::ToolExecution {
                server: self.name.clone(),
                tool: tool_name.into(),
                reason: format!("{e}"),
            })?;

        // Convert CallToolResult content to JSON value
        let content_values: Vec<Value> = result
            .content
            .iter()
            .filter_map(|content| {
                // Extract text content from the result
                if let Some(text) = content.as_text() {
                    // Try to parse as JSON first, fall back to string
                    match serde_json::from_str(text.text.as_ref()) {
                        Ok(v) => Some(v),
                        Err(_) => Some(Value::String(text.text.clone())),
                    }
                } else {
                    None
                }
            })
            .collect();

        // Return single value directly, or array if multiple
        let output = match content_values.len() {
            0 => Value::Null,
            1 => content_values.into_iter().next().unwrap_or(Value::Null),
            _ => Value::Array(content_values),
        };

        // If the tool call indicated an error, wrap it
        if result.is_error.unwrap_or(false) {
            return Err(MCPError::ToolExecution {
                server: self.name.clone(),
                tool: tool_name.into(),
                reason: format!("Tool returned error: {output}"),
            });
        }

        Ok(output)
    }
}
