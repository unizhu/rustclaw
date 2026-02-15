use crate::config::Config;
use anyhow::Result;
use rustclaw_channel::{create_default_tools, TelegramService};
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

        // Log agent config
        info!(
            "Agent config: max_tool_iterations={}, context_window={}, recent_turns={}",
            self.config.agent.max_tool_iterations,
            self.config.agent.context_window,
            self.config.agent.recent_turns
        );

        // Initialize persistence
        let persistence = PersistenceService::new(&self.config.database.path).await?;
        info!("Persistence service initialized");

        // Initialize provider based on config
        let provider = match self.config.providers.default.as_str() {
            "openai" => {
                if let Some(base_url) = &self.config.providers.openai.base_url {
                    if base_url.is_empty() {
                        Provider::openai(&self.config.providers.openai.model)
                    } else {
                        Provider::openai_with_base_url(&self.config.providers.openai.model, base_url)
                    }
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

        // Create tool registry with default tools (bash, file ops, etc.)
        let tools = create_default_tools();
        info!(
            "Tool registry initialized with {} tools",
            tools.get_tools().len()
        );

        // Create provider service with tools
        let provider_service = ProviderService::with_tools(provider, tools)
            .with_max_tool_iterations(self.config.agent.max_tool_iterations)
            .with_system_prompt(
                "You are a helpful AI assistant. You have access to tools for executing \
                 bash commands, reading files, and listing directories. Use these tools \
                 when the user asks you to perform system operations. \
                 \
                 CRITICAL TOOL CALLING RULES: \
                 1. When using a tool, output ONLY the tool call with valid JSON arguments \
                 2. NEVER add markdown code blocks, bash commands, or any text after tool calls \
                 3. Tool arguments must be pure JSON with no extra formatting \
                 4. Wait for the tool result before continuing \
                 \
                 Always be helpful and provide clear explanations."
            );
        info!("Provider service initialized");

        // Initialize Telegram channel
        let telegram_service = TelegramService::new(
            &self.config.telegram.bot_token,
            persistence,
            provider_service,
        );

        // Setup signal handler for graceful shutdown
        let shutdown = async {
            if let Err(e) = signal::ctrl_c().await {
                error!("Failed to install Ctrl+C handler: {}", e);
            }
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
