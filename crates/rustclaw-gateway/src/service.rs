use crate::config::Config;
use anyhow::Result;
use rustclaw_channel::TelegramService;
use rustclaw_logging;
use rustclaw_persistence::PersistenceService;
use rustclaw_provider::ProviderService;
use rustclaw_types::Provider;
use tokio::signal;
use tracing::{error, info, warn};

/// Gateway service - main orchestrator
pub struct GatewayService {
    config: Config,
}

impl GatewayService {
    /// Create a new gateway service
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Run the gateway service
    pub async fn run(self) -> Result<()> {
        // Initialize logging
        rustclaw_logging::init_logging(&self.config.logging.level)?;
        info!("Starting RustClaw Gateway Service");

        // Initialize persistence
        let persistence = PersistenceService::new(&self.config.database.path).await?;
        info!("Persistence service initialized");

        // Initialize provider based on config
        let provider = match self.config.providers.default.as_str() {
            "openai" => {
                if let Some(base_url) = &self.config.providers.openai.base_url {
                    Provider::openai_with_base_url(&self.config.providers.openai.model, base_url)
                } else {
                    Provider::openai(&self.config.providers.openai.model)
                }
            }
            "ollama" => Provider::ollama(
                &self.config.providers.ollama.model,
                &self.config.providers.ollama.base_url,
            ),
            _ => {
                warn!("Unknown provider, defaulting to OpenAI");
                Provider::default()
            }
        };
        let provider_service = ProviderService::new(provider);
        info!("Provider service initialized");

        // Initialize Telegram channel
        let telegram_service = TelegramService::new(
            &self.config.telegram.bot_token,
            persistence,
            provider_service,
        );

        // Setup signal handler for graceful shutdown
        let shutdown = async {
            signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
            info!("Received shutdown signal");
        };

        // Run the bot
        tokio::select! {
            result = telegram_service.run() => {
                if let Err(e) = result {
                    error!("Telegram service error: {}", e);
                }
            }
            _ = shutdown => {
                info!("Shutting down gracefully...");
            }
        }

        info!("Gateway service stopped");
        Ok(())
    }
}
