use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OpenAIConfig {
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
    /// Load configuration from file and environment variables
    pub fn load() -> anyhow::Result<Self> {
        // Load .env file
        dotenvy::dotenv().ok();

        // Load from rustclaw.toml or use defaults
        let mut config_builder = config::Config::builder()
            .add_source(config::File::with_name("rustclaw").required(false))
            .add_source(config::Environment::with_prefix("RUSTCLAW").separator("__"));

        // Build config
        let mut config = config_builder.build()?;

        // Override with specific env vars
        if let Ok(token) = env::var("TELEGRAM_BOT_TOKEN") {
            config.set("telegram__bot_token", token)?;
        }

        if let Ok(key) = env::var("OPENAI_API_KEY") {
            config.set("providers__openai__api_key", key)?;
        }

        if let Ok(url) = env::var("OPENAI_BASE_URL") {
            config.set("providers__openai__base_url", url)?;
        }

        if let Ok(url) = env::var("OLLAMA_BASE_URL") {
            config.set("providers__ollama__base_url", url)?;
        }

        let config: Self = config.try_deserialize()?;
        Ok(config)
    }
}
