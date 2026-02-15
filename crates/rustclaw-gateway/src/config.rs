use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Default config template created when no config exists
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

#[derive(Debug, Deserialize, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OpenAIConfig {
    #[allow(dead_code)]
    pub api_key: String,
    pub model: String,
    pub base_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OllamaConfig {
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProvidersConfig {
    pub default: String,
    pub openai: OpenAIConfig,
    pub ollama: OllamaConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AgentConfig {
    /// Maximum number of tool iterations per request
    #[serde(default = "default_max_tool_iterations")]
    pub max_tool_iterations: usize,
    
    /// Context window size in tokens
    #[serde(default = "default_context_window")]
    pub context_window: usize,
    
    /// Number of recent turns to keep before compression
    #[serde(default = "default_recent_turns")]
    pub recent_turns: usize,
}

fn default_max_tool_iterations() -> usize { 10 }
fn default_context_window() -> usize { 128_000 }
fn default_recent_turns() -> usize { 10 }

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_tool_iterations: default_max_tool_iterations(),
            context_window: default_context_window(),
            recent_turns: default_recent_turns(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub telegram: TelegramConfig,
    pub providers: ProvidersConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub mcp: rustclaw_mcp::MCPConfig,
}

impl Config {
    /// Get the global config path: ~/.rustclaw/rustclaw.toml
    fn global_config_path() -> PathBuf {
        dirs::home_dir()
            .expect("Could not find home directory")
            .join(".rustclaw")
            .join("rustclaw.toml")
    }

    /// Ensure global config directory and file exist, creating defaults if needed
    fn ensure_global_config() -> anyhow::Result<PathBuf> {
        let config_path = Self::global_config_path();
        let config_dir = config_path.parent().unwrap();

        if !config_dir.exists() {
            fs::create_dir_all(config_dir)?;
            eprintln!("Created config directory: {}", config_dir.display());
        }

        if !config_path.exists() {
            fs::write(&config_path, DEFAULT_CONFIG.trim())?;
            eprintln!("Created default config: {}", config_path.display());
            eprintln!("Please edit this file or set environment variables.");
        }

        Ok(config_path)
    }

    /// Load configuration with layered approach:
    /// 1. Global config: ~/.rustclaw/rustclaw.toml (auto-created if missing)
    /// 2. Local override: ./rustclaw.toml (workspace, optional)
    /// 3. Environment variables (highest priority)
    pub fn load() -> anyhow::Result<Self> {
        // Load .env file from current directory
        dotenvy::dotenv().ok();

        // Ensure global config exists
        let global_config_path = Self::ensure_global_config()?;

        // Build config with layered sources using builder pattern
        let mut builder = config::Config::builder()
            // Layer 1: Global config (required - we just created it if missing)
            .add_source(config::File::from(global_config_path))
            // Layer 2: Local workspace config (optional override)
            .add_source(config::File::with_name("rustclaw").required(false))
            // Layer 3: Environment variables with RUSTCLAW__ prefix
            .add_source(config::Environment::with_prefix("RUSTCLAW").separator("__"));

        // Layer 4: Apply convenience env var overrides (highest priority)
        if let Ok(token) = env::var("TELEGRAM_BOT_TOKEN") {
            builder = builder.set_override("telegram__bot_token", token)?;
        }

        if let Ok(key) = env::var("OPENAI_API_KEY") {
            builder = builder.set_override("providers__openai__api_key", key)?;
        }

        if let Ok(url) = env::var("OPENAI_BASE_URL") {
            builder = builder.set_override("providers__openai__base_url", url)?;
        }

        if let Ok(url) = env::var("OLLAMA_BASE_URL") {
            builder = builder.set_override("providers__ollama__base_url", url)?;
        }

        // Agent config overrides
        if let Ok(iterations) = env::var("RUSTCLAW_MAX_TOOL_ITERATIONS") {
            if let Ok(v) = iterations.parse::<i64>() {
                builder = builder.set_override("agent__max_tool_iterations", v)?;
            }
        }

        let config = builder.build()?;
        let config: Self = config.try_deserialize()?;
        Ok(config)
    }
}
