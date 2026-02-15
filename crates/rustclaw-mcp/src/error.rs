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
