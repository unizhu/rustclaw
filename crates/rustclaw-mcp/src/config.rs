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
    #[serde(default)]
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
    #[must_use]
    pub fn detect_transport(&self) -> TransportType {
        match self {
            MCPServerConfig::Simple(s) => {
                if s.starts_with("http://") || s.starts_with("https://") {
                    TransportType::HTTP(s.clone(), HashMap::new())
                } else {
                    TransportType::Stdio(s.clone())
                }
            }
            MCPServerConfig::Advanced { transport, .. } => match transport {
                TransportConfig::Stdio { command } => TransportType::Stdio(command.clone()),
                TransportConfig::HTTP { url, headers } => {
                    TransportType::HTTP(url.clone(), headers.clone())
                }
            },
        }
    }

    /// Get startup timeout (with fallback to global default)
    #[must_use]
    pub fn get_timeout(&self, global_timeout: u64) -> Duration {
        match self {
            MCPServerConfig::Simple(_) => Duration::from_secs(global_timeout),
            MCPServerConfig::Advanced {
                startup_timeout, ..
            } => Duration::from_secs(startup_timeout.unwrap_or(global_timeout)),
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
