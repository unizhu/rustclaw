//! `RustClaw` MCP Client Library
//!
//! Production-ready MCP (Model Context Protocol) client support for `RustClaw`.
//!
//! ## Features
//!
//! - Connect to MCP servers via stdio or Streamable HTTP transports
//! - Auto-negotiate protocol versions via `rmcp` SDK
//! - Discover and execute remote tools with full JSON Schema support
//! - Async startup with configurable timeouts
//! - Graceful error handling and degradation
//! - Bearer token authentication for remote servers
//! - Zero unsafe code

#![deny(unsafe_code, dead_code, unused_imports, unused_variables, missing_docs)]

pub mod client;
pub mod config;
pub mod error;
pub mod http_client;
pub mod registry;
pub mod tool_bridge;

pub use client::MCPClient;
pub use config::{MCPConfig, MCPServerConfig, TransportConfig};
pub use error::MCPError;
pub use registry::MCPToolRegistry;
pub use tool_bridge::MCPToolWrapper;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::{MCPClient, MCPConfig, MCPError, MCPServerConfig, MCPToolRegistry};
}
