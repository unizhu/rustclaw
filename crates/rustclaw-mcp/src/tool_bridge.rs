//! Tool bridge for MCP - bridges tools between different MCP servers

use crate::client::MCPClient;
use crate::error::{MCPError, Result};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Tool bridge that can route tool calls to different MCP servers
pub struct ToolBridge {
    /// Map of server name to MCP client
    clients: RwLock<HashMap<String, MCPClient>>,
}

impl ToolBridge {
    /// Create a new tool bridge
    #[must_use]
    pub fn new() -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
        }
    }

    /// Register an MCP client with the bridge
    pub async fn register(&self, name: String, client: MCPClient) {
        let mut clients = self.clients.write().await;
        clients.insert(name, client);
    }

    /// Execute a tool on a specific server
    ///
    /// # Errors
    /// Returns an error if the server or tool is not found, or if execution fails
    pub async fn execute(
        &self,
        server_name: &str,
        tool: &str,
        args: Value,
    ) -> Result<Value, MCPError> {
        let clients = self.clients.read().await;

        let client = clients
            .get(server_name)
            .ok_or_else(|| MCPError::ToolNotFound {
                server: server_name.into(),
                tool: tool.into(),
            })?;

        // call_tool is no longer async since it's a placeholder
        client.call_tool(tool, args)
    }

    /// List all available tools from all registered servers
    #[must_use]
    pub async fn list_tools(&self) -> Vec<(String, String)> {
        let clients = self.clients.read().await;
        let mut tools = Vec::new();

        for (server_name, client) in clients.iter() {
            for tool in &client.tools {
                tools.push((server_name.clone(), tool.name.clone()));
            }
        }

        tools
    }
}

impl Default for ToolBridge {
    fn default() -> Self {
        Self::new()
    }
}
