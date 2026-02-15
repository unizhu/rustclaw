//! MCP tool registry for managing multiple MCP clients

use crate::client::MCPClient;
use crate::config::MCPConfig;
use crate::error::MCPError;
use crate::tool_bridge::MCPToolWrapper;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinSet;
use tracing::{error, info};

/// Registry of MCP clients and their tools
pub struct MCPToolRegistry {
    /// Connected MCP clients (`server_name` → client)
    clients: Arc<RwLock<HashMap<String, MCPClient>>>,
}

impl MCPToolRegistry {
    /// Create an empty registry
    #[must_use] 
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Start all MCP servers configured in parallel
    pub async fn start_all(config: &MCPConfig) -> Self {
        let registry = Self::new();
        
        if config.servers.is_empty() {
            info!("No MCP servers configured");
            return registry;
        }
        
        info!("Starting {} MCP server(s)", config.servers.len());
        
        let mut tasks = JoinSet::new();
        
        // Spawn all clients concurrently
        for (name, server_config) in &config.servers {
            let name = name.clone();
            let config = server_config.clone();
            let timeout_secs = config.get_timeout(10).as_secs();
            let clients = Arc::clone(&registry.clients);
            
            tasks.spawn(async move {
                match MCPClient::start(
                    name.clone(),
                    &config,
                    std::time::Duration::from_secs(timeout_secs),
                )
                .await
                {
                    Ok(client) => {
                        info!(
                            "✅ MCP server '{}' started ({} tools, protocol {})",
                            name,
                            client.tools.len(),
                            client.protocol_version
                        );
                        clients.write().await.insert(name, client);
                    }
                    Err(e) => {
                        error!("❌ MCP server '{}' failed: {}", name, e);
                        // Graceful degradation: continue without this server
                    }
                }
            });
        }
        
        // Wait for all tasks to complete
        while tasks.join_next().await.is_some() {}
        
        let count = registry.clients.read().await.len();
        info!("MCP registry ready: {}/{} servers started", count, config.servers.len());
        
        registry
    }
    
    /// Execute a tool on a specific server
    pub async fn execute(
        &self,
        server_name: &str,
        tool_name: &str,
        args: Value,
    ) -> Result<Value, MCPError> {
        let clients = self.clients.read().await;
        
        let client = clients
            .get(server_name)
            .ok_or_else(|| MCPError::ToolNotFound {
                server: server_name.into(),
                tool: tool_name.into(),
            })?;
        
        client.call_tool(tool_name, args).await
    }
    
    /// Get all tools from all connected servers as `ToolFunction` wrappers
    #[must_use] 
    pub fn to_tool_functions(&self) -> Vec<Box<dyn rustclaw_provider::ToolFunction>> {
        let clients = self.clients.blocking_read();
        let mut tools = Vec::new();
        
        for (server_name, client) in clients.iter() {
            for mcp_tool in &client.tools {
                let wrapper = MCPToolWrapper {
                    server_name: server_name.clone(),
                    tool_name: mcp_tool.name.clone(),
                    full_name: format!("{}_{}", server_name, mcp_tool.name),
                    definition: mcp_tool.clone(),
                    registry: Arc::clone(&self.clients),
                };
                
                tools.push(Box::new(wrapper) as Box<dyn rustclaw_provider::ToolFunction>);
            }
        }
        
        tools
    }
    
    /// Check if registry is empty
    #[must_use] 
    pub fn is_empty(&self) -> bool {
        self.clients.blocking_read().is_empty()
    }
    
    /// Get number of connected servers
    #[must_use] 
    pub fn server_count(&self) -> usize {
        self.clients.blocking_read().len()
    }
    
    /// Get total tool count across all servers
    #[must_use] 
    pub fn tool_count(&self) -> usize {
        self.clients
            .blocking_read()
            .values()
            .map(|c| c.tools.len())
            .sum()
    }
}

impl Default for MCPToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
