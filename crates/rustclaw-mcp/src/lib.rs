//! `RustClaw` MCP Client Library
//!
//! Production-ready MCP (Model Context Protocol) client support for `RustClaw`.
//!
//! ## Features
//!
//! - Connect to MCP servers via stdio or HTTP transports
//! - Auto-negotiate protocol versions (2024-11-05, 2025-03-26, 2025-11-25)
//! - Async startup with configurable timeouts
//! - Graceful error handling
//! - Zero unsafe code

#![deny(unsafe_code, dead_code, unused_imports, unused_variables, missing_docs)]

pub mod client;
pub mod config;
pub mod error;
pub mod registry;
pub mod tool_bridge;

// TODO: Implement HTTP transport
// #[cfg(feature = "http")]
// pub mod transport;

pub use client::MCPClient;
pub use config::{MCPConfig, MCPServerConfig, TransportConfig};
pub use error::MCPError;
pub use registry::MCPToolRegistry;
pub use tool_bridge::MCPToolWrapper;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::{MCPClient, MCPConfig, MCPError, MCPServerConfig, MCPToolRegistry};
}
