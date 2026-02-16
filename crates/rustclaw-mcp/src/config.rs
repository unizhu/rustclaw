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
    /// HTTP transport (SSE or streaming)
    ///
    /// Matched first because it has a required `url` field that distinguishes
    /// it from Stdio. Serde tries variants top-to-bottom with `untagged`.
    HTTP {
        /// Server URL
        url: String,

        /// Optional HTTP headers (e.g. `Authorization`)
        #[serde(default)]
        headers: HashMap<String, String>,
    },

    /// stdio transport (launch subprocess)
    Stdio {
        /// Command to execute (e.g. `"npx"` or `"npx -y @modelcontextprotocol/server-filesystem /tmp"`)
        command: String,

        /// Optional separate arguments list
        ///
        /// If provided, `command` is treated as the program name only.
        /// If omitted, `command` is split on whitespace into program + args.
        #[serde(default)]
        args: Vec<String>,

        /// Optional environment variables to set for the child process
        #[serde(default)]
        env: HashMap<String, String>,
    },
}

/// Detected transport type with all parameters needed to start a connection
#[derive(Debug, Clone, PartialEq)]
pub enum TransportType {
    /// stdio transport: (program, args, env)
    Stdio {
        /// Program to execute
        program: String,
        /// Command-line arguments
        args: Vec<String>,
        /// Environment variables
        env: HashMap<String, String>,
    },
    /// HTTP transport: (url, headers)
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
                    // Split simple command string into program + args
                    let parts: Vec<&str> = s.split_whitespace().collect();
                    let program = parts
                        .first()
                        .map_or_else(|| s.clone(), |p| (*p).to_string());
                    let args: Vec<String> =
                        parts.iter().skip(1).map(|a| (*a).to_string()).collect();
                    TransportType::Stdio {
                        program,
                        args,
                        env: HashMap::new(),
                    }
                }
            }
            MCPServerConfig::Advanced { transport, .. } => match transport {
                TransportConfig::Stdio { command, args, env } => {
                    if args.is_empty() {
                        // No explicit args — split command string like Simple variant
                        let parts: Vec<&str> = command.split_whitespace().collect();
                        let program = parts
                            .first()
                            .map_or_else(|| command.clone(), |p| (*p).to_string());
                        let split_args: Vec<String> =
                            parts.iter().skip(1).map(|a| (*a).to_string()).collect();
                        TransportType::Stdio {
                            program,
                            args: split_args,
                            env: env.clone(),
                        }
                    } else {
                        // Explicit args — command is just the program name
                        TransportType::Stdio {
                            program: command.clone(),
                            args: args.clone(),
                            env: env.clone(),
                        }
                    }
                }
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

    /// Extract Authorization header value if present
    #[must_use]
    pub fn get_auth_header(&self) -> Option<String> {
        match self {
            MCPServerConfig::Simple(_) => None,
            MCPServerConfig::Advanced { transport, .. } => match transport {
                TransportConfig::HTTP { headers, .. } => headers.get("Authorization").cloned(),
                TransportConfig::Stdio { .. } => None,
            },
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_config_stdio() {
        let config = MCPServerConfig::Simple("npx -y server".into());
        assert_eq!(
            config.detect_transport(),
            TransportType::Stdio {
                program: "npx".into(),
                args: vec!["-y".into(), "server".into()],
                env: HashMap::new(),
            }
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
                args: Vec::new(),
                env: HashMap::new(),
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

    #[test]
    fn test_stdio_with_args_and_env() {
        let mut env = HashMap::new();
        env.insert("API_KEY".into(), "test_key".into());
        env.insert("MODE".into(), "TEST".into());

        let config = MCPServerConfig::Advanced {
            transport: TransportConfig::Stdio {
                command: "npx".into(),
                args: vec!["-y".into(), "@z_ai/mcp-server".into()],
                env: env.clone(),
            },
            startup_timeout: None,
        };

        assert_eq!(
            config.detect_transport(),
            TransportType::Stdio {
                program: "npx".into(),
                args: vec!["-y".into(), "@z_ai/mcp-server".into()],
                env,
            }
        );
    }

    #[test]
    fn test_stdio_command_split_with_env() {
        let mut env = HashMap::new();
        env.insert("KEY".into(), "value".into());

        let config = MCPServerConfig::Advanced {
            transport: TransportConfig::Stdio {
                command: "npx -y server".into(),
                args: Vec::new(),
                env: env.clone(),
            },
            startup_timeout: None,
        };

        assert_eq!(
            config.detect_transport(),
            TransportType::Stdio {
                program: "npx".into(),
                args: vec!["-y".into(), "server".into()],
                env,
            }
        );
    }

    #[test]
    fn test_toml_deserialization_stdio_with_env() {
        let toml_str = r#"
            [servers.zai]
            command = "npx"
            args = ["-y", "@z_ai/mcp-server"]
            env = { Z_AI_API_KEY = "test_key", Z_AI_MODE = "ZHIPU" }
        "#;

        let config: MCPConfig = toml::from_str(toml_str).expect("Failed to parse TOML");
        let server = config.servers.get("zai").expect("Server not found");

        match server.detect_transport() {
            TransportType::Stdio { program, args, env } => {
                assert_eq!(program, "npx");
                assert_eq!(args, vec!["-y", "@z_ai/mcp-server"]);
                assert_eq!(env.get("Z_AI_API_KEY").unwrap(), "test_key");
                assert_eq!(env.get("Z_AI_MODE").unwrap(), "ZHIPU");
            }
            _ => panic!("Expected Stdio transport"),
        }
    }
    #[test]
    fn test_http_headers_parsing() {
        let toml_str = r#"
            [servers.web-search]
            url = "https://example.com"
            headers = { Authorization = "Bearer token123" }
        "#;

        let config: MCPConfig = toml::from_str(toml_str).expect("Failed to parse TOML");
        let server = config.servers.get("web-search").expect("Server not found");

        match server.detect_transport() {
            TransportType::HTTP(url, headers) => {
                assert_eq!(url, "https://example.com");
                // Check if Authorization header is present and case-preserved
                assert_eq!(
                    headers.get("Authorization").map(|s| s.as_str()),
                    Some("Bearer token123")
                );
            }
            _ => panic!("Expected HTTP transport"),
        }
    }
}
