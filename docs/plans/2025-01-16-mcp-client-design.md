# MCP Client Integration Design

**Date:** 2025-01-XX  
**Author:** UGENT  
**Status:** Approved

## Overview

Add MCP (Model Context Protocol) client support to rustclaw, allowing it to connect to external MCP servers and expose their tools as native rustclaw tools for LLM use.

## Goals

1. **Tool Proxy Mode** - Connect to MCP servers and expose their tools to rustclaw's LLM
2. **Easy Configuration** - Simple TOML config with auto-detection
3. **Multiple Transports** - Support stdio, HTTP SSE, HTTP streaming
4. **Protocol Versions** - Auto-negotiate 2024-11-05, 2025-03-26, 2025-11-25
5. **Async Startup** - Non-blocking with configurable timeouts
6. **Production Quality** - Zero unsafe, no dead code, clippy-clean

## Architecture

### Component Structure

```
clawd/
├── crates/
│   ├── rustclaw-mcp/          # NEW CRATE
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs      # MCPClient wrapper around rmcp
│   │       ├── transport.rs   # HTTP SSE/Streaming transport impl
│   │       ├── tool_bridge.rs # MCP tools → rustclaw ToolFunction
│   │       └── registry.rs    # MCPToolRegistry
│   │
│   ├── rustclaw-provider/     # MODIFIED
│   │   └── src/lib.rs         # Add MCPToolRegistry integration
│   │
│   └── rustclaw-gateway/      # MODIFIED  
│       └── src/
│           ├── config.rs      # Add MCPConfig
│           └── service.rs     # Start MCP clients async
```

### Key Components

#### 1. MCPClient (`rustclaw-mcp/src/client.rs`)

Wrapper around official `rmcp` client.

```rust
pub struct MCPClient {
    name: String,
    inner: rmcp::Client,
    startup_timeout: Duration,
    tools: Vec<ToolDefinition>,
}

impl MCPClient {
    pub async fn start(config: &MCPServerConfig, timeout: Duration) -> Result<Self>;
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value>;
    pub fn get_tools(&self) -> &[ToolDefinition];
}
```

#### 2. MCPToolRegistry (`rustclaw-mcp/src/registry.rs`)

Manages multiple MCP clients.

```rust
pub struct MCPToolRegistry {
    clients: HashMap<String, MCPClient>,
}

impl MCPToolRegistry {
    pub async fn start_all(config: &MCPConfig) -> Result<Self>;
    pub fn get_all_tools(&self) -> Vec<Tool>;
    pub async fn execute(&self, server: &str, tool: &str, args: Value) -> Result<Value>;
}
```

#### 3. MCPToolBridge (`rustclaw-mcp/src/tool_bridge.rs`)

Wraps MCP tools as rustclaw ToolFunction.

```rust
pub struct MCPToolWrapper {
    server_name: String,
    tool_name: String,
    registry: Arc<MCPToolRegistry>,
}

impl ToolFunction for MCPToolWrapper {
    fn definition(&self) -> Tool;
    fn execute(&self, args: Value) -> Result<Value>;
}
```

### Dependencies

```toml
# rustclaw-mcp/Cargo.toml
[package]
name = "rustclaw-mcp"
version.workspace = true

[dependencies]
rmcp = { version = "0.8", features = ["client"] }
rustclaw-types = { path = "../rustclaw-types" }
tokio.workspace = true
anyhow.workspace = true
tracing.workspace = true
serde.workspace = true
serde_json.workspace = true
reqwest = { version = "0.12", features = ["json", "stream"], optional = true }
tokio-stream = "0.1"
futures.workspace = true
```

## Configuration

### TOML Structure

```toml
# rustclaw.toml

[mcp]
startup_timeout = 10  # global default (seconds)

[mcp.servers]
# Simple string form (auto-detect)
filesystem = "npx -y @modelcontextprotocol/server-filesystem /tmp"
weather = "http://localhost:3000/sse"

# Advanced form (override timeout)
slow_server = { 
  command = "some-slow-server --port 8080",
  startup_timeout = 30
}

http_with_auth = {
  url = "https://api.example.com/mcp",
  startup_timeout = 15,
  headers = { Authorization = "Bearer token123" }
}
```

### Rust Config Types

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct MCPConfig {
    #[serde(default = "default_startup_timeout")]
    pub startup_timeout: u64,
    
    pub servers: HashMap<String, MCPServerConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum MCPServerConfig {
    Simple(String),
    Advanced {
        #[serde(flatten)]
        transport: TransportConfig,
        #[serde(default)]
        startup_timeout: Option<u64>,
    },
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum TransportConfig {
    Stdio { command: String },
    HTTP { url: String, headers: HashMap<String, String> },
}
```

## Data Flow

### Startup Sequence

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Gateway Service Start                                     │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. Load Config (rustclaw.toml)                              │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. Start Core Services (sync, fast)                         │
│    - Database, Logging, Provider                            │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│ 4. Start MCP Clients (async, non-blocking)                 │
│    - Spawn background task for each MCP server             │
│    - Each with independent timeout                          │
│    - On failure: log error, continue                        │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│ 5. Build Tool Registries                                     │
│    - ToolRegistry (built-in tools)                          │
│    - MCPToolRegistry (from successfully started servers)   │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│ 6. Start Telegram Service (ready to accept messages)       │
└─────────────────────────────────────────────────────────────┘
```

### Async MCP Startup

```rust
// In service.rs
pub async fn run(self) -> Result<()> {
    // ... core services ...
    
    // Start MCP clients asynchronously (non-blocking)
    let mcp_registry = Arc::new(RwLock::new(MCPToolRegistry::new()));
    
    for (name, server_config) in &self.config.mcp.servers {
        let name = name.clone();
        let config = server_config.clone();
        let timeout = self.config.mcp.startup_timeout;
        let registry = mcp_registry.clone();
        
        tokio::spawn(async move {
            match MCPClient::start(&config, timeout).await {
                Ok(client) => {
                    info!("✅ MCP server '{}' started ({} tools)", 
                          name, client.tools.len());
                    registry.write().await.add_client(name, client);
                }
                Err(e) => {
                    error!("❌ MCP server '{}' failed: {}", name, e);
                }
            }
        });
    }
    
    // Don't wait! Continue immediately
    info!("Gateway ready (MCP servers starting in background)");
    
    // Start Telegram service
    telegram_service.run().await
}
```

### Tool Execution Flow

```
User Message → Telegram → Provider Service
                              ↓
                    LLM decides to call tool
                              ↓
                    Is it built-in or MCP tool?
                    ↙            ↘
          Built-in Tool      MCP Tool
                ↓                ↓
          ToolRegistry      MCPToolRegistry
                ↓                ↓
            Execute        Route to MCP server
                              ↓
                        rmcp::Client.call_tool()
                              ↓
                        Return result to LLM
                              ↓
                        LLM generates response
                              ↓
                        Send to Telegram user
```

## Tool Discovery & Naming

### Tool Discovery

When MCP server starts:
1. Initialize protocol (negotiate version)
2. List available tools
3. Store tool definitions

```rust
impl MCPClient {
    pub async fn start(config: &MCPServerConfig, timeout: Duration) -> Result<Self> {
        let client = rmcp::Client::connect(transport).await?;
        let init_result = client.initialize(/* ... */).await?;
        let tools_response = client.list_tools(None).await?;
        
        Ok(Self { client, tools: tools_response.tools })
    }
}
```

### Tool Naming

**Problem:** Multiple MCP servers might have tools with same name.

**Solution:** Prefix with server name.

```rust
impl MCPToolRegistry {
    pub fn to_tool_functions(&self) -> Vec<Box<dyn ToolFunction>> {
        let mut tools = Vec::new();
        
        for (server_name, client) in &self.clients {
            for mcp_tool in &client.tools {
                let rustclaw_tool = MCPToolWrapper {
                    server_name: server_name.clone(),
                    tool_name: mcp_tool.name.clone(),
                    full_name: format!("{}_{}", server_name, mcp_tool.name),
                };
                
                tools.push(Box::new(rustclaw_tool));
            }
        }
        
        tools
    }
}
```

**Example:**
```
Built-in tools:
  - echo
  - bash
  - read_file

MCP filesystem server:
  - filesystem_read_file
  - filesystem_write_file
  - filesystem_list_directory

MCP github server:
  - github_read_file
  - github_create_issue
  - github_search_code
```

## Transport Implementation

### Auto-Detection

```rust
impl MCPServerConfig {
    pub fn detect_transport(&self) -> TransportType {
        match self {
            MCPServerConfig::Simple(s) => {
                if s.starts_with("http://") || s.starts_with("https://") {
                    TransportType::HTTP(s.clone())
                } else {
                    TransportType::Stdio(s.clone())
                }
            }
            MCPServerConfig::Advanced { transport, .. } => {
                match transport {
                    TransportConfig::Stdio { command } => TransportType::Stdio(command.clone()),
                    TransportConfig::HTTP { url, .. } => TransportType::HTTP(url.clone()),
                }
            }
        }
    }
}
```

### HTTP SSE Transport

```rust
pub struct HTTPTransport {
    client: Client,
    endpoint: String,
    headers: HashMap<String, String>,
    tx: mpsc::Sender<String>,
    rx: mpsc::Receiver<String>,
}

impl HTTPTransport {
    pub async fn connect(url: String, headers: HashMap<String, String>) -> Result<Self> {
        let client = Client::new();
        
        // 1. GET /sse → establish SSE connection
        let response = client
            .get(&format!("{}/sse", url))
            .headers(headers.clone().try_into()?)
            .send()
            .await?;
        
        // 2. Extract message endpoint from "endpoint" event
        let endpoint = Self::parse_endpoint_event(response).await?;
        
        // 3. Create channels for bidirectional communication
        let (tx, rx) = mpsc::channel(100);
        
        Ok(Self { client, endpoint, headers, tx, rx })
    }
}
```

## Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum MCPError {
    #[error("Transport error: {0}")]
    Transport(String),
    
    #[error("Server '{server}' failed to start: {reason}")]
    StartupFailed { server: String, reason: String },
    
    #[error("Server '{server}' timeout after {timeout}s")]
    StartupTimeout { server: String, timeout: u64 },
    
    #[error("Tool '{tool}' not found on server '{server}'")]
    ToolNotFound { server: String, tool: String },
    
    #[error("Server '{server}' disconnected")]
    ServerDisconnected { server: String },
    
    #[error("Protocol error: {0}")]
    Protocol(String),
}
```

### Graceful Degradation

```rust
// On MCP server failure:
ERROR MCP server 'filesystem' failed: connection timeout
INFO  Continuing with 2/3 MCP servers (tools: echo, bash, weather)
```

## Code Quality

### Linting Rules

```rust
#![deny(
    unsafe_code,
    dead_code,
    unused_imports,
    unused_variables,
    missing_docs,
)]
```

### Clippy Configuration

```toml
[lints.clippy]
pedantic = "warn"
nursery = "warn"
unwrap_used = "deny"
expect_used = "deny"
```

### Testing

```rust
#[tokio::test]
async fn test_stdio_transport() {
    let config = MCPServerConfig::Simple(
        "npx -y @modelcontextprotocol/server-everything".into()
    );
    
    let client = MCPClient::start(&config, Duration::from_secs(30)).await
        .expect("Failed to start MCP server");
    
    assert!(!client.tools.is_empty());
}

#[tokio::test]
async fn test_startup_timeout() {
    let config = MCPServerConfig::Simple("sleep 9999".into());
    
    let result = MCPClient::start(&config, Duration::from_secs(1)).await;
    
    assert!(matches!(result, Err(MCPError::StartupTimeout { .. })));
}
```

## Implementation Phases

### Phase 1: Foundation
- Create `rustclaw-mcp` crate
- Implement MCPClient with stdio transport (via rmcp)
- Add MCPConfig to gateway

### Phase 2: Integration
- Implement MCPToolRegistry
- Create tool bridge (MCP → rustclaw)
- Integrate with ProviderService

### Phase 3: HTTP Transport
- Implement HTTP SSE transport
- Test with real HTTP MCP servers

### Phase 4: Quality & Polish
- Add comprehensive tests
- Fix all clippy warnings
- Documentation

## Success Criteria

✅ Can connect to stdio MCP servers  
✅ Can connect to HTTP SSE MCP servers  
✅ Tools appear as native rustclaw tools  
✅ Graceful degradation on server failure  
✅ Non-blocking startup  
✅ Auto-negotiate protocol version  
✅ Pass all quality gates (fmt, clippy, test)  
✅ Zero unsafe code  
✅ No dead code or unused imports  

## References

- [MCP Specification](https://modelcontextprotocol.io/)
- [Official rmcp SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [rust-mcp-sdk](https://github.com/rust-mcp-stack/rust-mcp-sdk)
- [MCP 2025-11-25 Update](https://workos.com/blog/mcp-2025-11-25-spec-update)
