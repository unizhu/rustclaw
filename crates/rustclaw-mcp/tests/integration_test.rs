//! Integration tests with real MCP servers

use rustclaw_mcp::{MCPClient, MCPConfig, MCPServerConfig, MCPToolRegistry};
use std::collections::HashMap;

#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored
async fn test_stdio_transport_with_real_server() {
    let config = MCPServerConfig::Simple(
        "npx -y @modelcontextprotocol/server-everything".into()
    );
    
    let client = MCPClient::start(
        "test".into(),
        &config,
        std::time::Duration::from_secs(30)
    ).await.expect("Failed to start MCP server");
    
    assert!(!client.tools.is_empty());
}

#[tokio::test]
async fn test_graceful_degradation() {
    let mut servers = HashMap::new();
    servers.insert(
        "invalid".into(),
        MCPServerConfig::Simple("invalid-command".into())
    );
    
    let config = MCPConfig {
        startup_timeout: 1,
        servers,
    };
    
    let registry = MCPToolRegistry::start_all(&config).await;
    assert_eq!(registry.server_count(), 0);
}

#[tokio::test]
async fn test_startup_timeout() {
    let config = MCPServerConfig::Simple("sleep 9999".into());
    
    let result = MCPClient::start(
        "timeout_test".into(),
        &config,
        std::time::Duration::from_secs(1)
    ).await;
    
    assert!(result.is_err());
}
