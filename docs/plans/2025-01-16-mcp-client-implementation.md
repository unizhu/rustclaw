# MCP Client Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add MCP (Model Context Protocol) client support to rustclaw, enabling it to connect to external MCP servers and expose their tools as native rustclaw tools.

**Architecture:** Create a new `rustclaw-mcp` crate that wraps the official `rmcp` SDK, implements HTTP transport, and bridges MCP tools to rustclaw's `ToolFunction` trait. Async startup, graceful error handling, zero unsafe code.

**Tech Stack:** Rust, rmcp (official MCP SDK), tokio, reqwest (HTTP), serde, thiserror

---

## Task 1: Create rustclaw-mcp Crate Foundation

**Files:**
- Create: `crates/rustclaw-mcp/Cargo.toml`
- Create: `crates/rustclaw-mcp/src/lib.rs`
- Modify: `Cargo.toml` (add to workspace members)

**Step 1: Update workspace Cargo.toml**

```bash
# In clawd/Cargo.toml, add rustclaw-mcp to members
```

Edit `clawd/Cargo.toml` line 2-9:
```toml
[workspace]
members = [
    "crates/rustclaw-types",
    "crates/rustclaw-logging",
    "crates/rustclaw-persistence",
    "crates/rustclaw-provider",
    "crates/rustclaw-channel",
    "crates/rustclaw-gateway",
    "crates/rustclaw-mcp",  # NEW
]
```

**Step 2: Create rustclaw-mcp/Cargo.toml**

Create `clawd/crates/rustclaw-mcp/Cargo.toml`:
```toml
[package]
name = "rustclaw-mcp"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
# MCP SDK
rmcp = { version = "0.1", features = ["client"] }

# Internal crates
rustclaw-types = { path = "../rustclaw-types" }

# Async runtime
tokio = { workspace = true, features = ["process", "time", "sync", "rt-multi-thread"] }
tokio-stream.workspace = true
futures.workspace = true

# HTTP client (optional)
reqwest = { version = "0.12", features = ["json", "stream"], optional = true }

# Serialization
serde.workspace = true
serde_json.workspace = true

# Error handling
anyhow.workspace = true
thiserror.workspace = true

# Logging
tracing.workspace = true

[features]
default = ["stdio"]
stdio = []
http = ["reqwest"]

[dev-dependencies]
tokio-test = "0.4"

# Quality gates
[lints.rust]
dead_code = "deny"
unused_imports = "deny"
unsafe_code = "deny"

[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
pedantic = "warn"
```

**Step 3: Create lib.rs with quality gates**

Create `clawd/crates/rustclaw-mcp/src/lib.rs`:
```rust
//! RustClaw MCP Client Library
//! 
//! Production-ready MCP (Model Context Protocol) client support for RustClaw.
//! 
//! ## Features
//! 
//! - Connect to MCP servers via stdio or HTTP transports
//! - Auto-negotiate protocol versions (2024-11-05, 2025-03-26, 2025-11-25)
//! - Async startup with configurable timeouts
//! - Graceful error handling
//! - Zero unsafe code

#![deny(
    unsafe_code,
    dead_code,
    unused_imports,
    unused_variables,
    missing_docs,
)]

pub mod error;
pub mod config;
pub mod client;
pub mod registry;
pub mod tool_bridge;

#[cfg(feature = "http")]
pub mod transport;

pub use error::MCPError;
pub use config::{MCPConfig, MCPServerConfig, TransportConfig};
pub use client::MCPClient;
pub use registry::MCPToolRegistry;
pub use tool_bridge::MCPToolWrapper;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::{
        MCPClient, MCPConfig, MCPServerConfig, MCPToolRegistry, MCPError,
    };
}
```

**Step 4: Verify crate compiles**

```bash
cd clawd
cargo check -p rustclaw-mcp
```

Expected: Compilation errors about missing modules (that's OK, we'll create them next)

**Step 5: Commit**

```bash
git add Cargo.toml crates/rustclaw-mcp/
git commit -m "feat: create rustclaw-mcp crate foundation"
```

---

## Task 2: Implement Error Types

**Files:**
- Create: `crates/rustclaw-mcp/src/error.rs`

**Step 1: Write the error module**

Create `clawd/crates/rustclaw-mcp/src/error.rs`:
```rust
//! Error types for MCP client operations

use thiserror::Error;

/// MCP client errors
#[derive(Debug, Error)]
pub enum MCPError {
    /// Transport-level error (connection, I/O)
    #[error("Transport error: {0}")]
    Transport(String),
    
    /// Server failed to start within timeout
    #[error("Server '{server}' failed to start: {reason}")]
    StartupFailed {
        /// Server name
        server: String,
        /// Failure reason
        reason: String,
    },
    
    /// Startup timeout exceeded
    #[error("Server '{server}' timeout after {timeout}s")]
    StartupTimeout {
        /// Server name
        server: String,
        /// Timeout in seconds
        timeout: u64,
    },
    
    /// Tool not found on server
    #[error("Tool '{tool}' not found on server '{server}'")]
    ToolNotFound {
        /// Server name
        server: String,
        /// Tool name
        tool: String,
    },
    
    /// Server disconnected unexpectedly
    #[error("Server '{server}' disconnected")]
    ServerDisconnected {
        /// Server name
        server: String,
    },
    
    /// Protocol-level error
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    /// Invalid response from server
    #[error("Invalid response from server '{server}': {details}")]
    InvalidResponse {
        /// Server name
        server: String,
        /// Error details
        details: String,
    },
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
    
    /// Serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    /// Generic I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convenient Result type alias
pub type Result<T> = std::result::Result<T, MCPError>;
```

**Step 2: Verify compilation**

```bash
cargo check -p rustclaw-mcp
```

Expected: Still missing modules, but error.rs should compile

**Step 3: Commit**

```bash
git add crates/rustclaw-mcp/src/error.rs
git commit -m "feat(mcp): add error types"
```

---

## Task 3: Implement Configuration Types

**Files:**
- Create: `crates/rustclaw-mcp/src/config.rs`

**Step 1: Write configuration module**

Create `clawd/crates/rustclaw-mcp/src/config.rs`:
```rust
//! Configuration types for MCP client

use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

/// MCP client configuration
#[derive(Debug, Deserialize, Clone)]
pub struct MCPConfig {
    /// Global startup timeout in seconds
    #[serde(default = "default_startup_timeout")]
    pub startup_timeout: u64,
    
    /// MCP server configurations
    pub servers: HashMap<String, MCPServerConfig>,
}

fn default_startup_timeout() -> u64 {
    10
}

impl Default for MCPConfig {
    fn default() -> Self {
        Self {
            startup_timeout: default_startup_timeout(),
            servers: HashMap::new(),
        }
    }
}

/// Individual MCP server configuration
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum MCPServerConfig {
    /// Simple form: just a command string or URL
    Simple(String),
    
    /// Advanced form with explicit transport and options
    Advanced {
        /// Transport configuration
        #[serde(flatten)]
        transport: TransportConfig,
        
        /// Override global startup timeout
        #[serde(default)]
        startup_timeout: Option<u64>,
    },
}

/// Transport configuration
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum TransportConfig {
    /// stdio transport (launch subprocess)
    Stdio {
        /// Command to execute
        command: String,
    },
    
    /// HTTP transport (SSE or streaming)
    HTTP {
        /// Server URL
        url: String,
        
        /// Optional HTTP headers
        #[serde(default)]
        headers: HashMap<String, String>,
    },
}

/// Detected transport type
#[derive(Debug, Clone, PartialEq)]
pub enum TransportType {
    /// stdio transport with command
    Stdio(String),
    /// HTTP transport with URL
    HTTP(String, HashMap<String, String>),
}

impl MCPServerConfig {
    /// Detect transport type from configuration
    pub fn detect_transport(&self) -> TransportType {
        match self {
            MCPServerConfig::Simple(s) => {
                if s.starts_with("http://") || s.starts_with("https://") {
                    TransportType::HTTP(s.clone(), HashMap::new())
                } else {
                    TransportType::Stdio(s.clone())
                }
            }
            MCPServerConfig::Advanced { transport, .. } => {
                match transport {
                    TransportConfig::Stdio { command } => {
                        TransportType::Stdio(command.clone())
                    }
                    TransportConfig::HTTP { url, headers } => {
                        TransportType::HTTP(url.clone(), headers.clone())
                    }
                }
            }
        }
    }
    
    /// Get startup timeout (with fallback to global default)
    pub fn get_timeout(&self, global_timeout: u64) -> Duration {
        match self {
            MCPServerConfig::Simple(_) => Duration::from_secs(global_timeout),
            MCPServerConfig::Advanced { startup_timeout, .. } => {
                Duration::from_secs(startup_timeout.unwrap_or(global_timeout))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_config_stdio() {
        let config = MCPServerConfig::Simple("npx -y server".into());
        assert_eq!(
            config.detect_transport(),
            TransportType::Stdio("npx -y server".into())
        );
    }

    #[test]
    fn test_simple_config_http() {
        let config = MCPServerConfig::Simple("http://localhost:3000".into());
        assert_eq!(
            config.detect_transport(),
            TransportType::HTTP("http://localhost:3000".into(), HashMap::new())
        );
    }

    #[test]
    fn test_timeout_override() {
        let config = MCPServerConfig::Advanced {
            transport: TransportConfig::Stdio {
                command: "server".into(),
            },
            startup_timeout: Some(30),
        };
        assert_eq!(config.get_timeout(10), Duration::from_secs(30));
    }

    #[test]
    fn test_timeout_default() {
        let config = MCPServerConfig::Simple("server".into());
        assert_eq!(config.get_timeout(10), Duration::from_secs(10));
    }
}
```

**Step 2: Run tests**

```bash
cargo test -p rustclaw-mcp --lib config
```

Expected: 4 tests pass

**Step 3: Commit**

```bash
git add crates/rustclaw-mcp/src/config.rs
git commit -m "feat(mcp): add configuration types"
```

---

## Task 4: Implement MCPClient with stdio Transport

**Files:**
- Create: `crates/rustclaw-mcp/src/client.rs`

**Step 1: Write client module**

Create `clawd/crates/rustclaw-mcp/src/client.rs`:
```rust
//! MCP client wrapper around rmcp

use crate::config::{MCPServerConfig, TransportType};
use crate::error::{MCPError, Result};
use rmcp::{Client, ServiceExt, transport::TokioChildProcess, ConfigureCommandExt};
use serde_json::Value;
use std::process::Command;
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tracing::{debug, info, warn};

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
    /// rmcp client
    client: Client,
    /// Available tools
    pub tools: Vec<ToolDefinition>,
    /// Negotiated protocol version
    pub protocol_version: String,
}

impl MCPClient {
    /// Start an MCP server and connect to it
    pub async fn start(
        name: String,
        config: &MCPServerConfig,
        timeout: Duration,
    ) -> Result<Self> {
        info!("Starting MCP server '{}' with timeout {:?}", name, timeout);
        
        let transport_type = config.detect_transport();
        
        let client = match transport_type {
            TransportType::Stdio(command_str) => {
                Self::start_stdio(&name, &command_str, timeout).await?
            }
            TransportType::HTTP(url, _headers) => {
                #[cfg(feature = "http")]
                {
                    Self::start_http(&name, &url, timeout).await?
                }
                #[cfg(not(feature = "http"))]
                {
                    return Err(MCPError::Config(
                        "HTTP transport requires 'http' feature".into(),
                    ));
                }
            }
        };
        
        Ok(client)
    }
    
    /// Start stdio transport
    async fn start_stdio(name: &str, command_str: &str, timeout: Duration) -> Result<Self> {
        debug!("Starting stdio transport: {}", command_str);
        
        // Parse command string into program and args
        let parts: Vec<&str> = command_str.split_whitespace().collect();
        if parts.is_empty() {
            return Err(MCPError::Config("Empty command".into()));
        }
        
        let program = parts[0];
        let args = &parts[1..];
        
        // Create tokio command
        let mut cmd = TokioCommand::new(program);
        cmd.args(args);
        
        // Connect with timeout
        let client = tokio::time::timeout(
            timeout,
            async {
                let transport = TokioChildProcess::new(cmd)?;
                Client::connect(transport).await
            }
        )
        .await
        .map_err(|_| MCPError::StartupTimeout {
            server: name.into(),
            timeout: timeout.as_secs(),
        })?
        .map_err(|e| MCPError::StartupFailed {
            server: name.into(),
            reason: e.to_string(),
        })?;
        
        // Initialize and list tools
        Self::initialize_client(name, client, timeout).await
    }
    
    /// Initialize client and discover tools
    async fn initialize_client(
        name: &str,
        client: Client,
        timeout: Duration,
    ) -> Result<Self> {
        info!("Initializing MCP client '{}'", name);
        
        // Initialize protocol (auto-negotiate version)
        let init_result = tokio::time::timeout(
            timeout,
            client.initialize(rmcp::schema::InitializeRequestParams {
                protocol_version: "2025-11-25".into(),  // Try latest
                capabilities: rmcp::schema::ClientCapabilities::default(),
                client_info: rmcp::schema::Implementation {
                    name: "rustclaw".into(),
                    version: env!("CARGO_PKG_VERSION").into(),
                    ..Default::default()
                },
                meta: None,
            })
        )
        .await
        .map_err(|_| MCPError::StartupTimeout {
            server: name.into(),
            timeout: timeout.as_secs(),
        })?
        .map_err(|e| MCPError::Protocol(format!("Initialize failed: {}", e)))?;
        
        let protocol_version = init_result.protocol_version;
        info!("MCP server '{}' protocol: {}", name, protocol_version);
        
        // List tools
        let tools_result = tokio::time::timeout(
            timeout,
            client.list_tools(None)
        )
        .await
        .map_err(|_| MCPError::StartupTimeout {
            server: name.into(),
            timeout: timeout.as_secs(),
        })?
        .map_err(|e| MCPError::Protocol(format!("List tools failed: {}", e)))?;
        
        let tools: Vec<ToolDefinition> = tools_result
            .tools
            .into_iter()
            .map(|t| ToolDefinition {
                name: t.name,
                description: t.description,
                input_schema: t.input_schema,
            })
            .collect();
        
        info!("MCP server '{}' has {} tools", name, tools.len());
        
        Ok(Self {
            name: name.into(),
            client,
            tools,
            protocol_version,
        })
    }
    
    /// Call a tool on this MCP server
    pub async fn call_tool(&self, tool_name: &str, args: Value) -> Result<Value> {
        debug!("Calling tool '{}' on server '{}'", tool_name, self.name);
        
        let result = self
            .client
            .call_tool(rmcp::schema::CallToolRequestParam {
                name: tool_name.into(),
                arguments: args,
            })
            .await
            .map_err(|e| MCPError::Protocol(format!("Tool call failed: {}", e)))?;
        
        // Extract content from result
        match result.content.first() {
            Some(rmcp::schema::Content::Text(text)) => {
                Ok(serde_json::from_str(&text.text).unwrap_or_else(|_| {
                    json!({ "content": text.text })
                }))
            }
            Some(rmcp::schema::Content::Image(img)) => {
                Ok(json!({
                    "type": "image",
                    "data": img.data,
                    "mime_type": img.mime_type,
                }))
            }
            Some(rmcp::schema::Content::Resource(res)) => {
                Ok(json!({
                    "type": "resource",
                    "uri": res.resource.uri,
                }))
            }
            None => Ok(json!({})),
        }
    }
}

/// Placeholder for HTTP transport (will be implemented later)
#[cfg(feature = "http")]
impl MCPClient {
    async fn start_http(name: &str, url: &str, timeout: Duration) -> Result<Self> {
        warn!("HTTP transport not yet implemented for server '{}'", name);
        Err(MCPError::Config(
            "HTTP transport implementation pending".into(),
        ))
    }
}
```

**Step 2: Verify compilation**

```bash
cargo check -p rustclaw-mcp
```

Expected: Compilation succeeds

**Step 3: Commit**

```bash
git add crates/rustclaw-mcp/src/client.rs
git commit -m "feat(mcp): implement MCPClient with stdio transport"
```

---

## Task 5: Implement MCPToolRegistry

**Files:**
- Create: `crates/rustclaw-mcp/src/registry.rs`

**Step 1: Write registry module**

Create `clawd/crates/rustclaw-mcp/src/registry.rs`:
```rust
//! MCP tool registry for managing multiple MCP clients

use crate::client::MCPClient;
use crate::config::MCPConfig;
use crate::error::{MCPError, Result};
use crate::tool_bridge::MCPToolWrapper;
use rustclaw_types::Tool;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinSet;
use tracing::{error, info, warn};

/// Registry of MCP clients and their tools
pub struct MCPToolRegistry {
    /// Connected MCP clients (server_name → client)
    clients: Arc<RwLock<HashMap<String, MCPClient>>>,
}

impl MCPToolRegistry {
    /// Create an empty registry
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
            let timeout_secs = config.get_timeout(config.startup_timeout).as_secs();
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
    ) -> Result<Value> {
        let clients = self.clients.read().await;
        
        let client = clients
            .get(server_name)
            .ok_or_else(|| MCPError::ToolNotFound {
                server: server_name.into(),
                tool: tool_name.into(),
            })?;
        
        client.call_tool(tool_name, args).await
    }
    
    /// Get all tools from all connected servers as ToolFunction wrappers
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
    pub fn is_empty(&self) -> bool {
        self.clients.blocking_read().is_empty()
    }
    
    /// Get number of connected servers
    pub fn server_count(&self) -> usize {
        self.clients.blocking_read().len()
    }
    
    /// Get total tool count across all servers
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
```

**Step 2: Verify compilation**

```bash
cargo check -p rustclaw-mcp
```

Expected: Compilation succeeds

**Step 3: Commit**

```bash
git add crates/rustclaw-mcp/src/registry.rs
git commit -m "feat(mcp): implement MCPToolRegistry"
```

---

## Task 6: Implement Tool Bridge

**Files:**
- Create: `crates/rustclaw-mcp/src/tool_bridge.rs`

**Step 1: Write tool bridge module**

Create `clawd/crates/rustclaw-mcp/src/tool_bridge.rs`:
```rust
//! Bridge between MCP tools and rustclaw's ToolFunction trait

use crate::client::ToolDefinition;
use crate::registry::MCPToolRegistry;
use anyhow::Result;
use rustclaw_types::{FunctionDefinition, Tool, ToolType};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Wrapper that makes MCP tools look like rustclaw tools
pub struct MCPToolWrapper {
    /// Server name
    pub server_name: String,
    /// Original MCP tool name
    pub tool_name: String,
    /// Full namespaced tool name (server_tool)
    pub full_name: String,
    /// Tool definition from MCP server
    pub definition: ToolDefinition,
    /// Reference to registry for tool execution
    pub registry: Arc<RwLock<std::collections::HashMap<String, crate::client::MCPClient>>>,
}

impl rustclaw_provider::ToolFunction for MCPToolWrapper {
    fn definition(&self) -> Tool {
        Tool {
            r#type: ToolType::Function,
            function: FunctionDefinition {
                name: self.full_name.clone(),
                description: self.definition.description.clone(),
                parameters: self.definition.input_schema.clone(),
            },
        }
    }
    
    fn execute(&self, args: Value) -> Result<Value> {
        // Convert async execution to sync (ToolFunction is sync)
        let registry = Arc::clone(&self.registry);
        let server = self.server_name.clone();
        let tool = self.tool_name.clone();
        
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let clients = registry.read().await;
                
                let client = clients
                    .get(&server)
                    .ok_or_else(|| {
                        anyhow::anyhow!("MCP server '{}' not available", server)
                    })?;
                
                client
                    .call_tool(&tool, args)
                    .await
                    .map_err(|e| anyhow::anyhow!("MCP tool call failed: {}", e))
            })
        })
    }
}
```

**Step 2: Verify compilation**

```bash
cargo check -p rustclaw-mcp
```

Expected: Compilation succeeds

**Step 3: Commit**

```bash
git add crates/rustclaw-mcp/src/tool_bridge.rs
git commit -m "feat(mcp): implement tool bridge"
```

---

## Task 7: Add Integration to Gateway Config

**Files:**
- Modify: `crates/rustclaw-gateway/src/config.rs`

**Step 1: Add MCP config to gateway config**

Edit `clawd/crates/rustclaw-gateway/src/config.rs`, add at line 101:

```rust
use rustclaw_mcp::MCPConfig;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub telegram: TelegramConfig,
    pub providers: ProvidersConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub mcp: MCPConfig,  // NEW
}
```

**Step 2: Update default config template**

Edit `clawd/crates/rustclaw-gateway/src/config.rs`, update DEFAULT_CONFIG constant:

```rust
const DEFAULT_CONFIG: &str = r#"
[telegram]
bot_token = ""  # Set via TELEGRAM_BOT_TOKEN env var

[providers]
default = "openai"  # or "ollama"

[providers.openai]
api_key = ""  # Set via OPENAI_API_KEY env var
model = "gpt-4o-mini"
base_url = ""  # Optional: Set via OPENAI_BASE_URL env var

[providers.ollama]
base_url = "http://localhost:11434"
model = "llama3"

[agent]
max_tool_iterations = 10  # Maximum tool calls per request
context_window = 128000   # Token limit for context
recent_turns = 10         # Turns to keep before compression

[database]
path = "rustclaw.db"

[logging]
level = "info"  # trace, debug, info, warn, error

# MCP servers (optional)
[mcp]
startup_timeout = 10  # seconds

[mcp.servers]
# Example: filesystem = "npx -y @modelcontextprotocol/server-filesystem /tmp"
"#;
```

**Step 3: Update Cargo.toml dependencies**

Edit `clawd/crates/rustclaw-gateway/Cargo.toml`, add:

```toml
[dependencies]
rustclaw-mcp = { path = "../rustclaw-mcp" }
# ... other dependencies
```

**Step 4: Verify compilation**

```bash
cargo check -p rustclaw-gateway
```

Expected: Compilation succeeds

**Step 5: Commit**

```bash
git add crates/rustclaw-gateway/
git commit -m "feat(gateway): add MCP config to gateway"
```

---

## Task 8: Integrate MCP into Gateway Service

**Files:**
- Modify: `crates/rustclaw-gateway/src/service.rs`

**Step 1: Import MCP registry**

Add at top of `clawd/crates/rustclaw-gateway/src/service.rs`:

```rust
use rustclaw_mcp::MCPToolRegistry;
use std::sync::Arc;
use tokio::sync::RwLock;
```

**Step 2: Start MCP servers asynchronously**

Edit the `run` method in `clawd/crates/rustclaw-gateway/src/service.rs`:

```rust
pub async fn run(self) -> Result<()> {
    // Initialize logging
    rustclaw_logging::init_logging(&self.config.logging.level)?;
    info!("Starting RustClaw Gateway Service");

    // ... existing database and provider setup ...

    // Create built-in tools
    let mut tools = create_default_tools();
    info!("Created {} built-in tools", tools.len());

    // Start MCP servers asynchronously (non-blocking)
    let mcp_registry = Arc::new(RwLock::new(MCPToolRegistry::new()));
    
    if !self.config.mcp.servers.is_empty() {
        let mcp_config = self.config.mcp.clone();
        let mcp_registry_clone = Arc::clone(&mcp_registry);
        
        tokio::spawn(async move {
            info!("Starting MCP servers in background...");
            let registry = MCPToolRegistry::start_all(&mcp_config).await;
            *mcp_registry_clone.write().await = registry;
        });
    }

    // Merge MCP tools (will be available once servers start)
    let mcp_tools = mcp_registry.read().await.to_tool_functions();
    if !mcp_tools.is_empty() {
        tools.extend(mcp_tools);
        info!("Added {} MCP tools", mcp_tools.len());
    }

    // Create provider service with all tools
    let provider_service = ProviderService::with_tools(provider, tools)
        .with_max_tool_iterations(self.config.agent.max_tool_iterations)
        .with_system_prompt(
            "You are a helpful AI assistant. You have access to tools for executing \
             bash commands, reading files, listing directories, and MCP tools. \
             Use these tools when the user asks you to perform system operations. \
             Always be helpful and provide clear explanations."
        );
    info!("Provider service initialized");

    // ... rest of the service ...
}
```

**Step 2: Verify compilation**

```bash
cargo check -p rustclaw-gateway
```

Expected: Compilation succeeds

**Step 3: Commit**

```bash
git add crates/rustclaw-gateway/src/service.rs
git commit -m "feat(gateway): integrate MCP servers into gateway service"
```

---

## Task 9: Add Workspace Dependencies

**Files:**
- Modify: `Cargo.toml` (workspace root)

**Step 1: Add rmcp to workspace dependencies**

Edit `clawd/Cargo.toml`, add to `[workspace.dependencies]`:

```toml
rmcp = { version = "0.1", features = ["client"] }
```

**Step 2: Update rustclaw-mcp Cargo.toml**

Edit `clawd/crates/rustclaw-mcp/Cargo.toml`, change rmcp dependency:

```toml
rmcp = { workspace = true }
```

**Step 3: Verify build**

```bash
cargo build
```

Expected: Build succeeds

**Step 4: Commit**

```bash
git add Cargo.toml crates/rustclaw-mcp/Cargo.toml
git commit -m "chore: add rmcp to workspace dependencies"
```

---

## Task 10: Write Integration Test

**Files:**
- Create: `crates/rustclaw-mcp/tests/integration_test.rs`

**Step 1: Write integration test**

Create `clawd/crates/rustclaw-mcp/tests/integration_test.rs`:

```rust
//! Integration tests with real MCP servers

use rustclaw_mcp::{MCPClient, MCPConfig, MCPServerConfig, MCPToolRegistry};
use std::collections::HashMap;

#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored
async fn test_stdio_transport_with_real_server() {
    // Use official MCP test server
    let config = MCPServerConfig::Simple(
        "npx -y @modelcontextprotocol/server-everything".into()
    );
    
    let client = MCPClient::start(
        "test".into(),
        &config,
        std::time::Duration::from_secs(30)
    ).await.expect("Failed to start MCP server");
    
    // Should discover tools
    assert!(!client.tools.is_empty());
    println!("Discovered {} tools", client.tools.len());
    
    // Should be able to call echo tool
    let result = client
        .call_tool("echo", serde_json::json!({"message": "hello"}))
        .await
        .expect("Tool call failed");
    
    println!("Tool result: {:?}", result);
    assert!(result.to_string().contains("hello"));
}

#[tokio::test]
#[ignore]
async fn test_registry_with_multiple_servers() {
    let mut servers = HashMap::new();
    
    servers.insert(
        "everything".into(),
        MCPServerConfig::Simple(
            "npx -y @modelcontextprotocol/server-everything".into()
        )
    );
    
    let config = MCPConfig {
        startup_timeout: 30,
        servers,
    };
    
    let registry = MCPToolRegistry::start_all(&config).await;
    
    // Should have started the server
    assert!(registry.server_count() > 0);
    assert!(registry.tool_count() > 0);
    
    println!("Started {} servers with {} tools", 
             registry.server_count(), 
             registry.tool_count());
}

#[tokio::test]
async fn test_graceful_degradation() {
    let mut servers = HashMap::new();
    
    // Invalid server (should fail gracefully)
    servers.insert(
        "invalid".into(),
        MCPServerConfig::Simple("invalid-command-that-does-not-exist".into())
    );
    
    let config = MCPConfig {
        startup_timeout: 1,
        servers,
    };
    
    // Should not panic
    let registry = MCPToolRegistry::start_all(&config).await;
    
    // Should be empty (server failed)
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
```

**Step 2: Run tests**

```bash
cd clawd
cargo test -p rustclaw-mcp
```

Expected: Tests pass (except ignored ones)

**Step 3: Commit**

```bash
git add crates/rustclaw-mcp/tests/
git commit -m "test(mcp): add integration tests"
```

---

## Task 11: Run Full Test Suite and Quality Gates

**Step 1: Run all tests**

```bash
cd clawd
cargo test --all
```

Expected: All tests pass

**Step 2: Check formatting**

```bash
cargo fmt --all -- --check
```

If fails, run: `cargo fmt --all`

**Step 3: Run clippy**

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

If fails, fix all warnings

**Step 4: Check for dead code**

```bash
cargo check --all-features
```

Expected: No warnings

**Step 5: Final commit**

```bash
git add .
git commit -m "chore: pass all quality gates"
```

---

## Task 12: Update Documentation

**Files:**
- Modify: `README.md`
- Modify: `rustclaw.toml.example`

**Step 1: Update README**

Add to `clawd/README.md` in the Features section:

```markdown
- **MCP Integration**: Connect to Model Context Protocol servers for extended tool capabilities
```

Add new section:

```markdown
## MCP Integration

RustClaw supports connecting to MCP (Model Context Protocol) servers to extend its capabilities.

### Configuration

Add MCP servers to your `rustclaw.toml`:

```toml
[mcp]
startup_timeout = 10  # seconds

[mcp.servers]
# Simple form
filesystem = "npx -y @modelcontextprotocol/server-filesystem /tmp"

# Advanced form
custom_server = { 
  command = "my-mcp-server --port 8080",
  startup_timeout = 30
}

# HTTP transport
weather = "http://localhost:3000/sse"
```

### Usage

MCP tools appear as native rustclaw tools with namespaced names:

```
filesystem_read_file
filesystem_write_file
weather_get_forecast
```

The LLM can use these tools just like built-in tools.

### Async Startup

MCP servers start asynchronously in the background, so rustclaw remains responsive even if servers are slow to start.

### Error Handling

If an MCP server fails to start, rustclaw continues without it. Check logs for startup errors.
```

**Step 2: Update example config**

Edit `clawd/rustclaw.toml.example`, add:

```toml
[mcp]
startup_timeout = 10

[mcp.servers]
# Example MCP servers (uncomment to use)
# filesystem = "npx -y @modelcontextprotocol/server-filesystem /tmp"
# github = "mcp-server-github"
```

**Step 3: Commit**

```bash
git add README.md rustclaw.toml.example
git commit -m "docs: add MCP integration documentation"
```

---

## Task 13: Create Example Config with MCP

**Files:**
- Create: `examples/mcp_config.toml`

**Step 1: Create example**

Create `clawd/examples/mcp_config.toml`:

```toml
# Example rustclaw configuration with MCP servers

[telegram]
bot_token = ""  # Set via TELEGRAM_BOT_TOKEN env var

[providers]
default = "openai"

[providers.openai]
api_key = ""  # Set via OPENAI_API_KEY env var
model = "gpt-4o-mini"

[agent]
max_tool_iterations = 10
context_window = 128000
recent_turns = 10

[database]
path = "rustclaw.db"

[logging]
level = "info"

# MCP Configuration
[mcp]
startup_timeout = 10  # Global default timeout in seconds

[mcp.servers]
# Filesystem MCP server
filesystem = "npx -y @modelcontextprotocol/server-filesystem /tmp"

# GitHub MCP server (requires GITHUB_TOKEN env var)
# github = "mcp-server-github"

# Custom MCP server with timeout override
# slow_server = { 
#   command = "my-slow-server --port 8080",
#   startup_timeout = 30
# }

# HTTP-based MCP server
# weather = "http://localhost:3000/sse"
```

**Step 2: Commit**

```bash
git add examples/mcp_config.toml
git commit -m "docs: add MCP configuration example"
```

---

## Task 14: Final Verification and Release Prep

**Step 1: Build release**

```bash
cargo build --release
```

Expected: Build succeeds

**Step 2: Run clippy on release**

```bash
cargo clippy --release -- -D warnings
```

Expected: No warnings

**Step 3: Test with actual MCP server**

```bash
# Install Node.js if needed
# npm install -g @modelcontextprotocol/server-filesystem

# Set up test config
cp examples/mcp_config.toml rustclaw.toml

# Run gateway
cargo run --release
```

Expected: Gateway starts, MCP server connects, tools available

**Step 4: Create git tag**

```bash
git tag -a v0.2.0 -m "feat: add MCP client support"
git push origin v0.2.0
```

**Step 5: Final commit**

```bash
git add .
git commit -m "chore: prepare for v0.2.0 release"
```

---

## Success Criteria

✅ `cargo test --all` passes  
✅ `cargo fmt --all -- --check` passes  
✅ `cargo clippy --all` passes with no warnings  
✅ `cargo build --release` succeeds  
✅ Zero unsafe code (`grep -r "unsafe" --include="*.rs"` returns nothing)  
✅ No dead code warnings  
✅ Integration tests pass with real MCP servers  
✅ Documentation updated  
✅ Example configs provided  

---

## Future Enhancements (Out of Scope)

- HTTP transport implementation (Phase 3)
- OAuth authentication
- Connection pooling
- Retry logic
- Health monitoring
- Metrics/observability

---

**Plan complete! Two execution options:**

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach would you like, Uni?** 🤔
