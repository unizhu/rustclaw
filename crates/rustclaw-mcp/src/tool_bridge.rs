//! Bridge between MCP tools and rustclaw's `ToolFunction` trait

use crate::client::{MCPClient, ToolDefinition};
use anyhow::Result;
use rustclaw_types::Tool;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Wrapper that makes MCP tools look like rustclaw tools
pub struct MCPToolWrapper {
    /// Server name
    pub server_name: String,
    /// Original MCP tool name
    pub tool_name: String,
    /// Full namespaced tool name (`server_tool`)
    pub full_name: String,
    /// Tool definition from MCP server
    pub definition: ToolDefinition,
    /// Reference to registry for tool execution
    pub registry: Arc<RwLock<std::collections::HashMap<String, MCPClient>>>,
}

impl rustclaw_provider::ToolFunction for MCPToolWrapper {
    fn definition(&self) -> Tool {
        Tool::function(
            &self.full_name,
            self.definition
                .description
                .as_deref()
                .unwrap_or("No description"),
            self.definition.input_schema.clone(),
        )
    }

    fn execute(&self, args: Value) -> Result<Value> {
        // Convert async call_tool to sync (ToolFunction trait is sync)
        let registry = Arc::clone(&self.registry);
        let server = self.server_name.clone();
        let tool = self.tool_name.clone();

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let clients = registry.read().await;

                let client = clients
                    .get(&server)
                    .ok_or_else(|| anyhow::anyhow!("MCP server '{server}' not available"))?;

                client
                    .call_tool(&tool, args)
                    .await
                    .map_err(|e| anyhow::anyhow!("MCP tool call failed: {e}"))
            })
        })
    }
}
