use crate::config::Config;
use anyhow::Result;
use rustclaw_channel::{create_default_tools, TelegramService};
use rustclaw_mcp::MCPToolRegistry;
use rustclaw_persistence::PersistenceService;
use rustclaw_provider::ProviderService;
use rustclaw_skills::SkillsRegistry;
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
                let model = &self.config.providers.openai.model;
                let api_key = self
                    .config
                    .providers
                    .openai
                    .api_key
                    .as_ref()
                    .filter(|k| !k.is_empty());
                let base_url = self
                    .config
                    .providers
                    .openai
                    .base_url
                    .as_ref()
                    .filter(|u| !u.is_empty());

                // Use full constructor if we have API key and/or base URL
                match (api_key, base_url) {
                    (Some(key), Some(url)) => Provider::openai_full(model, key, url),
                    (Some(key), None) => Provider::openai_with_api_key(model, key),
                    (None, Some(url)) => Provider::openai_with_base_url(model, url),
                    (None, None) => Provider::openai(model),
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
            "Tool registry initialized with {} built-in tools",
            tools.get_tools().len()
        );

        // Initialize MCP servers and wait for tools
        let mcp_tools_list = if !self.config.mcp.servers.is_empty() {
            info!("Initializing MCP servers...");
            let registry = MCPToolRegistry::start_all(&self.config.mcp).await;

            // Convert to tool functions
            let tools = registry.to_tool_functions().await;
            info!("MCP initialized with {} tools", tools.len());

            // Keep registry for reference if needed (currently we just need tools)
            // mcp_registry = registry;
            tools
        } else {
            Vec::new()
        };

        // Initialize skills system with progressive disclosure
        let mut skills_registry = SkillsRegistry::new();

        // Add configured skills directories
        for dir in &self.config.skills.directories {
            // Expand tilde to home directory
            let expanded_path = if dir.starts_with('~') {
                if let Some(home) = dirs::home_dir() {
                    dir.replacen('~', home.to_str().unwrap(), 1)
                } else {
                    dir.clone()
                }
            } else {
                dir.clone()
            };
            skills_registry = skills_registry.add_directory(expanded_path);
        }

        // Discover skills (Phase 1: Load metadata only)
        if let Err(e) = skills_registry.discover() {
            warn!("Failed to discover skills: {}", e);
        } else {
            info!("Discovered {} skills", skills_registry.len());
        }

        // Generate skills list for system prompt
        let skills_prompt = skills_registry.generate_system_prompt();

        // Create provider service with tools
        let base_prompt = "You are a helpful AI assistant. You have access to tools for executing \
                 bash commands, reading files, and listing directories. Use these tools \
                 when the user asks you to perform system operations. \
                 \
                 CRITICAL TOOL CALLING RULES: \
                 1. When using a tool, output ONLY the tool call with valid JSON arguments \
                 2. NEVER add markdown code blocks, bash commands, or any text after tool calls \
                 3. Tool arguments must be pure JSON with no extra formatting \
                 4. Wait for the tool result before continuing \
                 \
                 Always be helpful and provide clear explanations.";

        let full_prompt = format!("{}{}", base_prompt, skills_prompt);

        // Initialize provider service with ALL tools
        let mut provider_service = ProviderService::new(provider)
            .with_tool_registry(tools) // Starts with default tools
            .with_max_tool_iterations(self.config.agent.max_tool_iterations)
            .with_system_prompt(full_prompt);

        // Register MCP tools
        for tool in mcp_tools_list {
            provider_service.tools_mut().register(tool);
        }
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
