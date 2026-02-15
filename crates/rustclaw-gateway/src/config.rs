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

[database]
path = "rustclaw.db"

[logging]
level = "info"  # trace, debug, info, warn, error
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
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
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

        // Build config with layered sources (later sources override earlier ones)
        let mut config_builder = config::Config::builder()
            // Layer 1: Global config (required - we just created it if missing)
            .add_source(config::File::from(global_config_path))
            // Layer 2: Local workspace config (optional override)
            .add_source(config::File::with_name("rustclaw").required(false))
            // Layer 3: Environment variables with RUSTCLAW__ prefix
            .add_source(config::Environment::with_prefix("RUSTCLAW").separator("__"));

        // Layer 4: Apply convenience env var overrides (highest priority)
        if let Ok(token) = env::var("TELEGRAM_BOT_TOKEN") {
            config_builder = config_builder.set_override("telegram__bot_token", token)?;
        }

        if let Ok(key) = env::var("OPENAI_API_KEY") {
            config_builder = config_builder.set_override("providers__openai__api_key", key)?;
        }

        if let Ok(url) = env::var("OPENAI_BASE_URL") {
            config_builder = config_builder.set_override("providers__openai__base_url", url)?;
        }

        if let Ok(url) = env::var("OLLAMA_BASE_URL") {
            config_builder = config_builder.set_override("providers__ollama__base_url", url)?;
        }

        let config = config_builder.build()?;

        let config: Self = config.try_deserialize()?;
        Ok(config)
    }
}
