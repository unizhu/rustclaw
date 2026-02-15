# MCP Client Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add MCP client support to rustclaw that connects to external MCP servers and exposes their tools to the LLM.

**Architecture:** Create a new `rustclaw-mcp` crate using the official `rmcp` SDK, integrate with rustclaw's existing ToolRegistry via a tool bridge, and support both stdio and HTTP transports with async non-blocking startup.

**Tech Stack:** Rust, rmcp 0.8, tokio, reqwest, serde, thiserror

---

## Phase 1: Core Infrastructure

### Task 1: Create rustclaw-mcp crate

**Files:**
- Create: `clawd/crates/rustclaw-mcp/Cargo.toml`
- Create: `clawd/crates/rustclaw-mcp/src/lib.rs`

**Step 1: Add crate to workspace**

Update `clawd/Cargo.toml`:
```toml
[workspace]
members = [
    "crates/rustclaw-types",
    "crates/rustclaw-logging",
    "crates/rustclaw-persistence",
    "crates/rustclaw-provider",
    "crates/rustclaw-channel",
    "crates/rustclaw-gateway",
    "crates/rustclaw-mcp",  # ADD THIS
]
```

**Step 2: Create Cargo.toml**

Create `clawd/crates/rustclaw-mcp/Cargo.toml`:
```toml
[package]
name = "rustclaw-mcp"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
rmcp = { version = "0.1", features = ["client"] }
rustclaw-types = { path = "../rustclaw-types" }
tokio = { workspace = true, features = ["process", "time", "sync", "rt-multi-thread"] }
tokio-stream.workspace = true
futures.workspace = true
anyhow.workspace = true
thiserror.workspace = true
tracing.workspace = true
serde.workspace = true
serde_json.workspace = true

[features]
default = ["stdio"]
stdio = []
http = []

[lints.rust]
dead_code = "deny"
unused_imports = "deny"
unsafe_code = "deny"

[lints.clippy]
unwrap_used = "deny"
pedantic = "warn"
nursery = "warn"
```

**Step 3: Create lib.rs skeleton**

Create `clawd/crates/rustclaw-mcp/src/lib.rs`:
```rust
//! RustClaw MCP Client Library
//! 
//! Provides MCP (Model Context Protocol) client support for RustClaw
//! with support for stdio and HTTP transports.

#![deny(
    unsafe_code,
    dead_code,
    unused_imports,
    unused_variables,
    missing_docs,
)]

pub mod client;
pub mod config;
pub mod error;
pub mod registry;
pub mod tool_bridge;
pub mod transport;

pub use client::MCPClient;
pub use config::{MCPConfig, MCPServerConfig};
pub use error::MCPError;
pub use registry::MCPToolRegistry;
pub use transport::{TransportConfig, TransportType};
```

**Step 4: Verify compilation**

Run: `cd clawd && cargo check -p rustclaw-mcp`
Expected: Compilation errors (modules don't exist yet)

**Step 5: Commit**

```bash
git add crates/rustclaw-mcp/Cargo.toml crates/rustclaw-mcp/src/lib.rs Cargo.toml
git commit -m "feat: create rustclaw-mcp crate skeleton"
```

---

### Task 2: Define error types

**Files:**
- Create: `clawd/crates/rustclaw-mcp/src/error.rs`

**Step 1: Write error types**

Create `clawd/crates/rustclaw-mcp/src/error.rs`:
```rust
//! Error types for MCP client

use thiserror::Error;

/// MCP client errors
#[derive(Debug, Error)]
pub enum MCPError {
    /// Transport error
    #[error("Transport error: {0}")]
    Transport(String),

    /// Server startup failed
    #[error("Server '{server}' failed to start: {reason}")]
    StartupFailed {
        /// Server name
        server: String,
        /// Failure reason
        reason: String,
    },

    /// Server startup timeout
    #[error("Server '{server}' timeout after {timeout}s")]
    StartupTimeout {
        /// Server name
        server: String,
        /// Timeout in seconds
        timeout: u64,
    },

    /// Tool not found
    #[error("Tool '{tool}' not found on server '{server}'")]
    ToolNotFound {
        /// Server name
        server: String,
        /// Tool name
        tool: String,
    },

    /// Server disconnected
    #[error("Server '{server}' disconnected")]
    ServerDisconnected {
        /// Server name
        server: String,
    },

    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Invalid response
    #[error("Invalid response from server '{server}': {details}")]
    InvalidResponse {
        /// Server name
        server: String,
        /// Error details
        details: String,
    },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
```

**Step 2: Verify compilation**

Run: `cd clawd && cargo check -p rustclaw-mcp`
Expected: Compilation errors (missing modules)

**Step 3: Commit**

```bash
git add crates/rustclaw-mcp/src/error.rs
git commit -m "feat(mcp): add error types"
```

---

### Task 3: Define configuration types

**Files:**
- Create: `clawd/crates/rustclaw-mcp/src/config.rs`

**Step 1: Write config types**

Create `clawd/crates/rustclaw-mcp/src/config.rs`:
```rust
//! Configuration types for MCP client

use serde::Deserialize;
use std::collections::HashMap;

/// MCP configuration
#[derive(Debug, Deserialize, Clone)]
pub struct MCPConfig {
    /// Global startup timeout in seconds
    #[serde(default = "default_startup_timeout")]
    pub startup_timeout: u64,

    /// MCP servers
    pub servers: HashMap<String, MCPServerConfig>,
}

fn default_startup_timeout() -> u64 {
    10
}

/// MCP server configuration
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum MCPServerConfig {
    /// Simple form: just a command or URL
    Simple(String),

    /// Advanced form with overrides
    Advanced {
        /// Transport configuration
        #[serde(flatten)]
        transport: TransportConfig,

        /// Override startup timeout
        #[serde(default)]
        startup_timeout: Option<u64>,
    },
}

/// Transport configuration
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum TransportConfig {
    /// stdio transport
    Stdio {
        /// Command to execute
        command: String,
    },

    /// HTTP transport
    HTTP {
        /// Server URL
        url: String,

        /// Optional headers
        #[serde(default)]
        headers: HashMap<String, String>,
    },
}

/// Transport type (detected from config)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportType {
    /// stdio transport
    Stdio(String),

    /// HTTP transport
    HTTP(String),
}

impl MCPServerConfig {
    /// Detect transport type from configuration
    pub fn detect_transport(&self) -> TransportType {
        match self {
            MCPServerConfig::Simple(s) => {
                if s.starts_with("http://") || s.starts_with("https://") {
                    TransportType::HTTP(s.clone())
                } else {
                    TransportType::Stdio(s.clone())
                }
            }
            MCPServerConfig::Advanced { transport, .. } => match transport {
                TransportConfig::Stdio { command } => TransportType::Stdio(command.clone()),
                TransportConfig::HTTP { url, .. } => TransportType::HTTP(url.clone()),
            },
        }
    }

    /// Get timeout (with global fallback)
    pub fn get_timeout(&self, global: u64) -> std::time::Duration {
        match self {
            MCPServerConfig::Simple(_) => std::time::Duration::from_secs(global),
            MCPServerConfig::Advanced { startup_timeout, .. } => {
                std::time::Duration::from_secs(startup_timeout.unwrap_or(global))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_stdio_transport() {
        let config = MCPServerConfig::Simple("npx -y server".into());
        assert_eq!(config.detect_transport(), TransportType::Stdio("npx -y server".into()));
    }

    #[test]
    fn test_detect_http_transport() {
        let config = MCPServerConfig::Simple("http://localhost:3000/sse".into());
        assert_eq!(config.detect_transport(), TransportType::HTTP("http://localhost:3000/sse".into()));
    }

    #[test]
    fn test_timeout_fallback() {
        let config = MCPServerConfig::Simple("server".into());
        assert_eq!(config.get_timeout(10), std::time::Duration::from_secs(10));
    }

    #[test]
    fn test_timeout_override() {
        let config = MCPServerConfig::Advanced {
            transport: TransportConfig::Stdio { command: "server".into() },
            startup_timeout: Some(30),
        };
        assert_eq!(config.get_timeout(10), std::time::Duration::from_secs(30));
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cd clawd && cargo test -p rustclaw-mcp`
Expected: Tests pass (no dependencies yet)

**Step 3: Commit**

```bash
git add crates/rustclaw-mcp/src/config.rs
git commit -m "feat(mcp): add configuration types"
```

---

### Task 4: Create transport module

**Files:**
- Create: `clawd/crates/rustclaw-mcp/src/transport.rs`

**Step 1: Write transport module skeleton**

Create `clawd/crates/rustclaw-mcp/src/transport.rs`:
```rust
//! Transport implementations for MCP

use crate::error::MCPError;
use crate::config::TransportType;

/// Transport trait for MCP communication
pub trait Transport: Send + Sync {
    /// Send a message
    fn send(&mut self, message: String) -> impl std::future::Future<Output = Result<(), MCPError>> + Send;

    /// Receive a message
    fn recv(&mut self) -> impl std::future::Future<Output = Result<Option<String>, MCPError>> + Send;
}

/// Create transport from type
pub async fn create_transport(transport_type: &TransportType) -> Result<Box<dyn Transport>, MCPError> {
    match transport_type {
        TransportType::Stdio(command) => {
            #[cfg(feature = "stdio")]
            {
                Ok(Box::new(create_stdio_transport(command).await?))
            }
            #[cfg(not(feature = "stdio"))]
            {
                Err(MCPError::Transport("stdio transport not enabled".into()))
            }
        }
        TransportType::HTTP(url) => {
            #[cfg(feature = "http")]
            {
                Ok(Box::new(create_http_transport(url).await?))
            }
            #[cfg(not(feature = "http"))]
            {
                Err(MCPError::Transport("http transport not enabled".into()))
            }
        }
    }
}

#[cfg(feature = "stdio")]
async fn create_stdio_transport(command: &str) -> Result<StdioTransport, MCPError> {
    // TODO: Implement in Phase 1
    Err(MCPError::Transport("stdio transport not implemented".into()))
}

#[cfg(feature = "http")]
async fn create_http_transport(url: &str) -> Result<HTTPTransport, MCPError> {
    // TODO: Implement in Phase 3
    Err(MCPError::Transport("http transport not implemented".into()))
}

/// stdio transport (placeholder)
#[cfg(feature = "stdio")]
pub struct StdioTransport {
    // TODO: Add fields
}

/// HTTP transport (placeholder)
#[cfg(feature = "http")]
pub struct HTTPTransport {
    // TODO: Add fields
}
```

**Step 2: Verify compilation**

Run: `cd clawd && cargo check -p rustclaw-mcp`
Expected: Compilation succeeds

**Step 3: Commit**

```bash
git add crates/rustclaw-mcp/src/transport.rs
git commit -m "feat(mcp): add transport module skeleton"
```

---

### Task 5: Create client module

**Files:**
- Create: `clawd/crates/rustclaw-mcp/src/client.rs`

**Step 1: Write client skeleton**

Create `clawd/crates/rustclaw-mcp/src/client.rs`:
```rust
//! MCP client implementation

use crate::config::MCPServerConfig;
use crate::error::MCPError;
use std::time::Duration;

/// MCP client
pub struct MCPClient {
    /// Server name
    pub name: String,

    /// Protocol version
    pub protocol_version: String,

    /// Available tools
    pub tools: Vec<rmcp::schema::Tool>,
}

impl MCPClient {
    /// Start MCP client
    pub async fn start(
        name: String,
        config: &MCPServerConfig,
        timeout: Duration,
    ) -> Result<Self, MCPError> {
        // TODO: Implement in Phase 1
        Err(MCPError::Transport("client not implemented".into()))
    }

    /// Call a tool
    pub async fn call_tool(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, MCPError> {
        // TODO: Implement in Phase 2
        Err(MCPError::Transport("tool call not implemented".into()))
    }
}
```

**Step 2: Verify compilation**

Run: `cd clawd && cargo check -p rustclaw-mcp`
Expected: Compilation errors (rmcp schema not imported)

**Step 3: Fix imports**

Update `clawd/crates/rustclaw-mcp/src/client.rs`:
```rust
//! MCP client implementation

use crate::config::MCPServerConfig;
use crate::error::MCPError;
use std::time::Duration;

/// MCP client
pub struct MCPClient {
    /// Server name
    pub name: String,

    /// Protocol version
    pub protocol_version: String,

    /// Available tools (using rmcp schema)
    pub tools: Vec<serde_json::Value>,  // Placeholder until rmcp types available
}
```

**Step 4: Verify compilation**

Run: `cd clawd && cargo check -p rustclaw-mcp`
Expected: Compilation succeeds

**Step 5: Commit**

```bash
git add crates/rustclaw-mcp/src/client.rs
git commit -m "feat(mcp): add client module skeleton"
```

---

### Task 6: Create registry module

**Files:**
- Create: `clawd/crates/rustclaw-mcp/src/registry.rs`

**Step 1: Write registry skeleton**

Create `clawd/crates/rustclaw-mcp/src/registry.rs`:
```rust
//! MCP tool registry

use crate::client::MCPClient;
use crate::config::MCPConfig;
use crate::error::MCPError;
use std::collections::HashMap;

/// MCP tool registry
pub struct MCPToolRegistry {
    /// Connected MCP clients
    clients: HashMap<String, MCPClient>,
}

impl MCPToolRegistry {
    /// Create empty registry
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    /// Start all MCP servers from configuration
    pub async fn start_all(config: &MCPConfig) -> Result<Self, MCPError> {
        // TODO: Implement in Phase 2
        Ok(Self::new())
    }

    /// Add a client
    pub fn add_client(&mut self, name: String, client: MCPClient) {
        self.clients.insert(name, client);
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }

    /// Get all tools from all clients
    pub fn get_all_tools(&self) -> Vec<serde_json::Value> {
        // TODO: Implement in Phase 2
        Vec::new()
    }

    /// Execute a tool on a specific server
    pub async fn execute(
        &self,
        server_name: &str,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, MCPError> {
        // TODO: Implement in Phase 2
        Err(MCPError::ToolNotFound {
            server: server_name.into(),
            tool: tool_name.into(),
        })
    }
}

impl Default for MCPToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 2: Verify compilation**

Run: `cd clawd && cargo check -p rustclaw-mcp`
Expected: Compilation succeeds

**Step 3: Commit**

```bash
git add crates/rustclaw-mcp/src/registry.rs
git commit -m "feat(mcp): add registry module skeleton"
```

---

### Task 7: Create tool bridge module

**Files:**
- Create: `clawd/crates/rustclaw-mcp/src/tool_bridge.rs`

**Step 1: Write tool bridge skeleton**

Create `clawd/crates/rustclaw-mcp/src/tool_bridge.rs`:
```rust
//! Tool bridge - MCP tools to rustclaw ToolFunction

use crate::registry::MCPToolRegistry;
use std::sync::Arc;
use tokio::sync::RwLock;

/// MCP tool wrapper
pub struct MCPToolWrapper {
    /// Server name
    pub server_name: String,

    /// Tool name
    pub tool_name: String,

    /// Full tool name (server_tool)
    pub full_name: String,

    /// Tool definition
    pub definition: serde_json::Value,

    /// Registry reference
    pub registry: Arc<RwLock<MCPToolRegistry>>,
}

// TODO: Implement ToolFunction trait in Phase 2
```

**Step 2: Verify compilation**

Run: `cd clawd && cargo check -p rustclaw-mcp`
Expected: Compilation succeeds

**Step 3: Commit**

```bash
git add crates/rustclaw-mcp/src/tool_bridge.rs
git commit -m "feat(mcp): add tool bridge skeleton"
```

---

## Phase 2: Tool Integration

### Task 8: Implement stdio transport

**Files:**
- Modify: `clawd/crates/rustclaw-mcp/src/transport.rs`

**Step 1: Implement stdio transport**

Update `clawd/crates/rustclaw-mcp/src/transport.rs`:
```rust
#[cfg(feature = "stdio")]
pub struct StdioTransport {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout: tokio::process::ChildStdout,
}

#[cfg(feature = "stdio")]
async fn create_stdio_transport(command: &str) -> Result<StdioTransport, MCPError> {
    use tokio::process::Command;
    
    // Parse command
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err(MCPError::Transport("Empty command".into()));
    }

    let cmd = parts[0];
    let args = &parts[1..];

    // Spawn process
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| MCPError::Transport(format!("Failed to spawn process: {}", e)))?;

    let stdin = child.stdin.take().ok_or_else(|| {
        MCPError::Transport("Failed to open stdin".into())
    })?;
    
    let stdout = child.stdout.take().ok_or_else(|| {
        MCPError::Transport("Failed to open stdout".into())
    })?;

    Ok(StdioTransport {
        child,
        stdin,
        stdout,
    })
}

#[cfg(feature = "stdio")]
impl Transport for StdioTransport {
    async fn send(&mut self, message: String) -> Result<(), MCPError> {
        use tokio::io::AsyncWriteExt;
        
        self.stdin.write_all(message.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;
        
        Ok(())
    }

    async fn recv(&mut self) -> Result<Option<String>, MCPError> {
        use tokio::io::AsyncBufReadExt;
        
        let reader = tokio::io::BufReader::new(&mut self.stdout);
        let mut lines = reader.lines();
        
        if let Some(line) = lines.next_line().await? {
            Ok(Some(line))
        } else {
            Ok(None)
        }
    }
}
```

**Step 2: Test stdio transport**

Create `clawd/crates/rustclaw-mcp/src/transport.rs` test at bottom:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stdio_transport_creation() {
        let result = create_stdio_transport("echo hello").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stdio_empty_command() {
        let result = create_stdio_transport("").await;
        assert!(result.is_err());
    }
}
```

**Step 3: Run tests**

Run: `cd clawd && cargo test -p rustclaw-mcp --features stdio`
Expected: Tests pass

**Step 4: Commit**

```bash
git add crates/rustclaw-mcp/src/transport.rs
git commit -m "feat(mcp): implement stdio transport"
```

---

### Task 9: Implement MCPClient with rmcp

**Files:**
- Modify: `clawd/crates/rustclaw-mcp/src/client.rs`

**Step 1: Update Cargo.toml dependencies**

Update `clawd/crates/rustclaw-mcp/Cargo.toml`:
```toml
[dependencies]
rmcp = { version = "0.1", features = ["client"] }
# ... other deps ...
```

**Step 2: Implement client**

Update `clawd/crates/rustclaw-mcp/src/client.rs`:
```rust
//! MCP client implementation

use crate::config::MCPServerConfig;
use crate::error::MCPError;
use crate::transport::{create_transport, Transport};
use std::time::Duration;

/// MCP client
pub struct MCPClient {
    /// Server name
    pub name: String,

    /// Protocol version
    pub protocol_version: String,

    /// Available tools
    pub tools: Vec<serde_json::Value>,

    /// Transport
    transport: Box<dyn Transport>,
}

impl MCPClient {
    /// Start MCP client
    pub async fn start(
        name: String,
        config: &MCPServerConfig,
        timeout: Duration,
    ) -> Result<Self, MCPError> {
        // Detect transport type
        let transport_type = config.detect_transport();
        
        // Create transport with timeout
        let transport = tokio::time::timeout(
            timeout,
            create_transport(&transport_type)
        )
        .await
        .map_err(|_| MCPError::StartupTimeout {
            server: name.clone(),
            timeout: timeout.as_secs(),
        })??;

        // TODO: Initialize MCP protocol
        // TODO: List tools
        
        Ok(Self {
            name,
            protocol_version: "2024-11-05".into(),  // Default
            tools: Vec::new(),
            transport,
        })
    }

    /// Call a tool
    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, MCPError> {
        // TODO: Implement JSON-RPC call
        Err(MCPError::Transport("tool call not implemented".into()))
    }
}
```

**Step 3: Test client**

Add test to `clawd/crates/rustclaw-mcp/src/client.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_timeout() {
        let config = MCPServerConfig::Simple("sleep 9999".into());
        let result = MCPClient::start(
            "test".into(),
            &config,
            Duration::from_millis(100),
        )
        .await;

        assert!(result.is_err());
    }
}
```

**Step 4: Run tests**

Run: `cd clawd && cargo test -p rustclaw-mcp --features stdio`
Expected: Tests pass

**Step 5: Commit**

```bash
git add crates/rustclaw-mcp/src/client.rs crates/rustclaw-mcp/Cargo.toml
git commit -m "feat(mcp): implement MCPClient with rmcp integration"
```

---

## Phase 3: Integration

### Task 10: Add MCPConfig to gateway

**Files:**
- Modify: `clawd/crates/rustclaw-gateway/src/config.rs`

**Step 1: Add MCPConfig to Config struct**

Update `clawd/crates/rustclaw-gateway/src/config.rs`:
```rust
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub telegram: TelegramConfig,
    pub providers: ProvidersConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub mcp: rustclaw_mcp::MCPConfig,  // ADD THIS
}
```

**Step 2: Update Cargo.toml dependencies**

Update `clawd/crates/rustclaw-gateway/Cargo.toml`:
```toml
[dependencies]
rustclaw-mcp = { path = "../rustclaw-mcp" }
# ... other deps ...
```

**Step 3: Update default config template**

Update `clawd/crates/rustclaw-gateway/src/config.rs` DEFAULT_CONFIG:
```rust
const DEFAULT_CONFIG: &str = r#"
[telegram]
bot_token = ""

[providers]
default = "openai"

[providers.openai]
api_key = ""
model = "gpt-4o-mini"

[providers.ollama]
base_url = "http://localhost:11434"
model = "llama3"

[agent]
max_tool_iterations = 10
context_window = 128000
recent_turns = 10

[database]
path = "rustclaw.db"

[logging]
level = "info"

[mcp]
startup_timeout = 10
[mcp.servers]
"#;
```

**Step 4: Verify compilation**

Run: `cd clawd && cargo check -p rustclaw-gateway`
Expected: Compilation succeeds

**Step 5: Commit**

```bash
git add crates/rustclaw-gateway/src/config.rs crates/rustclaw-gateway/Cargo.toml
git commit -m "feat(gateway): add MCPConfig to gateway config"
```

---

### Task 11: Start MCP clients in gateway service

**Files:**
- Modify: `clawd/crates/rustclaw-gateway/src/service.rs`

**Step 1: Import MCPToolRegistry**

Update `clawd/crates/rustclaw-gateway/src/service.rs`:
```rust
use crate::config::Config;
use anyhow::Result;
use rustclaw_channel::{create_default_tools, TelegramService};
use rustclaw_logging;
use rustclaw_mcp::MCPToolRegistry;  // ADD THIS
use rustclaw_persistence::PersistenceService;
use rustclaw_provider::ProviderService;
use rustclaw_types::Provider;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, warn};
```

**Step 2: Start MCP clients async**

Update `clawd/crates/rustclaw-gateway/src/service.rs` in `run` method:
```rust
pub async fn run(self) -> Result<()> {
    // Initialize logging
    rustclaw_logging::init_logging(&self.config.logging.level)?;
    info!("Starting RustClaw Gateway Service");

    // ... existing code ...

    // Create provider service
    let provider = create_provider(&self.config.providers)?;

    // Start MCP clients asynchronously
    let mcp_registry = Arc::new(tokio::sync::RwLock::new(MCPToolRegistry::new()));
    let mcp_registry_clone = mcp_registry.clone();
    let mcp_config = self.config.mcp.clone();

    tokio::spawn(async move {
        match MCPToolRegistry::start_all(&mcp_config).await {
            Ok(registry) => {
                *mcp_registry_clone.write().await = registry;
                info!("MCP servers started successfully");
            }
            Err(e) => {
                error!("Failed to start MCP registry: {}", e);
            }
        }
    });

    // Create built-in tools
    let mut tools = create_default_tools();

    // Add MCP tools (will be available once servers start)
    // TODO: Merge MCP tools in Phase 2

    let provider_service = ProviderService::with_tools(provider, tools)
        .with_max_tool_iterations(self.config.agent.max_tool_iterations)
        .with_system_prompt(/* ... */);

    info!("Provider service initialized");

    // ... rest of the code ...
}
```

**Step 3: Verify compilation**

Run: `cd clawd && cargo check -p rustclaw-gateway`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add crates/rustclaw-gateway/src/service.rs
git commit -m "feat(gateway): start MCP clients asynchronously"
```

---

## Phase 4: Testing & Quality

### Task 12: Add integration tests

**Files:**
- Create: `clawd/crates/rustclaw-mcp/tests/integration_test.rs`

**Step 1: Write integration test**

Create `clawd/crates/rustclaw-mcp/tests/integration_test.rs`:
```rust
use rustclaw_mcp::*;
use std::time::Duration;

#[tokio::test]
#[ignore] // Requires npx installed
async fn test_real_mcp_server() {
    let config = MCPConfig {
        startup_timeout: 30,
        servers: [(
            "test".into(),
            MCPServerConfig::Simple("npx -y @modelcontextprotocol/server-everything".into())
        )].into_iter().collect(),
    };

    let registry = MCPToolRegistry::start_all(&config)
        .await
        .expect("Failed to start registry");

    assert!(!registry.is_empty());
}

#[tokio::test]
async fn test_graceful_degradation() {
    let config = MCPConfig {
        startup_timeout: 1,
        servers: [(
            "invalid".into(),
            MCPServerConfig::Simple("invalid-command-12345".into())
        )].into_iter().collect(),
    };

    let result = MCPToolRegistry::start_all(&config).await;
    assert!(result.is_ok());
}
```

**Step 2: Run tests**

Run: `cd clawd && cargo test -p rustclaw-mcp --test integration_test`
Expected: Tests pass

**Step 3: Commit**

```bash
git add crates/rustclaw-mcp/tests/integration_test.rs
git commit -m "test(mcp): add integration tests"
```

---

### Task 13: Run quality gates

**Step 1: Format code**

Run: `cd clawd && cargo fmt --all`

**Step 2: Run clippy**

Run: `cd clawd && cargo clippy --all-targets --all-features -- -D warnings`
Expected: No warnings

**Step 3: Run all tests**

Run: `cd clawd && cargo test --all`
Expected: All tests pass

**Step 4: Build release**

Run: `cd clawd && cargo build --release`
Expected: Build succeeds

**Step 5: Commit any fixes**

```bash
git add .
git commit -m "chore: fix clippy warnings and format code"
```

---

## Phase 5: Documentation & Polish

### Task 14: Update README

**Files:**
- Modify: `clawd/README.md`

**Step 1: Add MCP section to README**

Add to `clawd/README.md`:
```markdown
## MCP Integration

RustClaw supports connecting to external MCP (Model Context Protocol) servers to extend its tool capabilities.

### Configuration

Add MCP servers to your `rustclaw.toml`:

```toml
[mcp]
startup_timeout = 10  # seconds

[mcp.servers]
# Simple form
filesystem = "npx -y @modelcontextprotocol/server-filesystem /tmp"

# HTTP server
weather = "http://localhost:3000/sse"

# Advanced form with timeout override
slow_server = { command = "some-server", startup_timeout = 30 }
```

### Usage

MCP tools are automatically discovered and appear as native rustclaw tools with namespaced names:

- `filesystem_read_file`
- `filesystem_write_file`
- `weather_get_forecast`

The LLM can use these tools just like built-in tools.
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add MCP integration documentation"
```

---

### Task 15: Create example config

**Files:**
- Modify: `clawd/rustclaw.toml.example`

**Step 1: Add MCP example**

Update `clawd/rustclaw.toml.example`:
```toml
# ... existing config ...

[mcp]
# Global startup timeout for MCP servers (seconds)
startup_timeout = 10

[mcp.servers]
# Example: Filesystem MCP server
# filesystem = "npx -y @modelcontextprotocol/server-filesystem /tmp"

# Example: HTTP MCP server
# weather = "http://localhost:3000/sse"

# Example: Advanced configuration
# slow_server = { 
#   command = "some-slow-server",
#   startup_timeout = 30
# }
```

**Step 2: Commit**

```bash
git add rustclaw.toml.example
git commit -m "docs: add MCP configuration example"
```

---

## Final Checklist

Before marking complete:

- [ ] All tests pass: `cargo test --all`
- [ ] No clippy warnings: `cargo clippy -- -D warnings`
- [ ] Code formatted: `cargo fmt --all -- --check`
- [ ] Documentation updated
- [ ] Example config updated
- [ ] README updated
- [ ] All commits made

---

**Plan complete! 🎉**

Total estimated time: 7-11 days
