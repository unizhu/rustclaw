//! MCP client wrapper around rmcp

use crate::config::{MCPServerConfig, TransportType};
use crate::error::{MCPError, Result};
use serde_json::Value;
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tracing::{debug, info};

#[cfg(feature = "http")]
use tracing::warn;

/// MCP tool definition
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: Option<String>,
    /// Input schema (JSON Schema)
    pub input_schema: Value,
}

/// MCP client wrapper
pub struct MCPClient {
    /// Server name
    pub name: String,
    /// Available tools
    pub tools: Vec<ToolDefinition>,
    /// Negotiated protocol version
    pub protocol_version: String,
    /// Server process (for stdio transport)
    #[allow(dead_code)] // Will be used when we implement shutdown
    process: Option<tokio::process::Child>,
}

impl MCPClient {
    /// Start an MCP server and connect to it
    ///
    /// # Errors
    /// Returns an error if the server fails to start or times out
    pub async fn start(name: String, config: &MCPServerConfig, timeout: Duration) -> Result<Self> {
        info!("Starting MCP server '{}' with timeout {:?}", name, timeout);

        let transport_type = config.detect_transport();

        let client = match transport_type {
            TransportType::Stdio(command_str) => {
                Self::start_stdio(&name, &command_str, timeout).await?
            }
            TransportType::HTTP(url, _headers) => {
                #[cfg(feature = "http")]
                {
                    Self::start_http(&name, &url, timeout)?
                }
                #[cfg(not(feature = "http"))]
                {
                    let _ = url; // Silence unused variable warning
                    return Err(MCPError::Config(
                        "HTTP transport requires 'http' feature".into(),
                    ));
                }
            }
        };

        Ok(client)
    }

    /// Start stdio transport
    async fn start_stdio(name: &str, command_str: &str, _timeout: Duration) -> Result<Self> {
        debug!("Starting stdio transport: {}", command_str);

        // Parse command string into program and args
        let parts: Vec<&str> = command_str.split_whitespace().collect();
        if parts.is_empty() {
            return Err(MCPError::Config("Empty command".into()));
        }

        let program = parts[0];
        let args = &parts[1..];

        // Try to spawn the process
        let mut cmd = TokioCommand::new(program);
        cmd.args(args);

        // Attempt to spawn the process
        let child = cmd.spawn().map_err(|e| MCPError::StartupFailed {
            server: name.into(),
            reason: format!("Failed to spawn '{program}': {e}"),
        })?;

        // Wait a bit to see if the process stays alive
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Initialize client (simulated)
        info!("MCP server '{}' started (simulated)", name);

        Ok(Self {
            name: name.into(),
            tools: Vec::new(), // Will be populated when we integrate rmcp
            protocol_version: "2024-11-05".into(),
            process: Some(child),
        })
    }

    /// Call a tool on this MCP server
    ///
    /// # Errors
    /// Returns an error if the tool call fails or is not implemented
    pub fn call_tool(&self, tool_name: &str, _args: Value) -> Result<Value> {
        debug!("Calling tool '{}' on server '{}'", tool_name, self.name);

        // Simulated for now - will implement with rmcp
        Err(MCPError::Protocol(
            "Tool calls not yet implemented - awaiting rmcp integration".into(),
        ))
    }
}

/// Placeholder for HTTP transport (will be implemented later)
#[cfg(feature = "http")]
impl MCPClient {
    fn start_http(name: &str, _url: &str, _timeout: Duration) -> Result<Self> {
        warn!("HTTP transport not yet implemented for server '{}'", name);
        Err(MCPError::Config(
            "HTTP transport implementation pending".into(),
        ))
    }
}
