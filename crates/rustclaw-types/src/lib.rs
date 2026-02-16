//! RustClaw Types - Core types for the RustClaw gateway
//!
//! This module defines the core data types used throughout the application.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A user in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub telegram_user_id: i64,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

impl User {
    pub fn new(id: i64) -> Self {
        Self {
            id,
            telegram_user_id: id,
            username: None,
            first_name: None,
            last_name: None,
        }
    }

    pub fn with_telegram(
        id: i64,
        username: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
    ) -> Self {
        Self {
            id,
            telegram_user_id: id,
            username,
            first_name,
            last_name,
        }
    }
}

/// Content of a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    Text(String),
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub chat_id: i64,
    pub sender: User,
    pub content: MessageContent,
    pub timestamp: DateTime<Utc>,
}

impl Message {
    pub fn new(chat_id: i64, user: User, content: MessageContent) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            chat_id,
            sender: user,
            content,
            timestamp: Utc::now(),
        }
    }
}

/// LLM Provider configuration
#[derive(Debug, Clone)]
pub enum Provider {
    OpenAI {
        model: String,
        api_key: Option<String>,
        base_url: Option<String>,
    },
    Ollama {
        model: String,
        base_url: String,
    },
}

impl Default for Provider {
    fn default() -> Self {
        Provider::OpenAI {
            model: "gpt-4o-mini".to_string(),
            api_key: None,
            base_url: None,
        }
    }
}

impl Provider {
    pub fn openai(model: &str) -> Self {
        Provider::OpenAI {
            model: model.to_string(),
            api_key: None,
            base_url: None,
        }
    }

    pub fn openai_with_base_url(model: &str, base_url: &str) -> Self {
        Provider::OpenAI {
            model: model.to_string(),
            api_key: None,
            base_url: Some(base_url.to_string()),
        }
    }

    pub fn openai_with_api_key(model: &str, api_key: &str) -> Self {
        Provider::OpenAI {
            model: model.to_string(),
            api_key: Some(api_key.to_string()),
            base_url: None,
        }
    }

    pub fn openai_full(model: &str, api_key: &str, base_url: &str) -> Self {
        Provider::OpenAI {
            model: model.to_string(),
            api_key: Some(api_key.to_string()),
            base_url: Some(base_url.to_string()),
        }
    }

    pub fn ollama(model: &str, base_url: &str) -> Self {
        Provider::Ollama {
            model: model.to_string(),
            base_url: base_url.to_string(),
        }
    }
}

// ============================================================================
// Tool Calling Types (OpenAI-compatible)
// ============================================================================

/// A tool definition following OpenAI's function calling schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDef,
}

impl Tool {
    pub fn function(name: &str, description: &str, parameters: serde_json::Value) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: name.to_string(),
                description: description.to_string(),
                parameters,
                strict: Some(true),
            },
        }
    }
}

/// Function definition within a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// A tool call from the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

/// Function call details within a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String, // JSON-encoded arguments
}

impl ToolCall {
    /// Parse the arguments as a specific type
    pub fn parse_args<T: for<'de> Deserialize<'de>>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.function.arguments)
    }
}

/// Result of executing a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub output: String,
}

impl ToolResult {
    pub fn new(tool_call_id: String, output: impl Into<String>) -> Self {
        Self {
            tool_call_id,
            output: output.into(),
        }
    }

    pub fn from_json(tool_call_id: String, value: &impl Serialize) -> Self {
        Self {
            tool_call_id,
            output: serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string()),
        }
    }
}

/// Response from a completion that may include tool calls
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: String,
}

impl CompletionResponse {
    pub fn text(content: String) -> Self {
        Self {
            content: Some(content),
            tool_calls: vec![],
            finish_reason: "stop".to_string(),
        }
    }

    pub fn tool_calls(calls: Vec<ToolCall>) -> Self {
        Self {
            content: None,
            tool_calls: calls,
            finish_reason: "tool_calls".to_string(),
        }
    }

    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

/// Chat role for messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A chat message for the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: Some(content.into()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: Some(content.into()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: Some(content.into()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant_with_tools(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: Role::Assistant,
            content,
            name: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: Some(content.into()),
            name: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}
