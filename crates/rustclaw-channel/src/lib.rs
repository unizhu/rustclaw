use anyhow::{anyhow, Result};
use rustclaw_persistence::PersistenceService;
use rustclaw_provider::ProviderService;
use rustclaw_types::{Message as RustClawMessage, MessageContent, User};
use std::sync::Arc;
use teloxide::{error_handlers::LoggingErrorHandler, prelude::*, utils::command::BotCommands};
use tokio::sync::RwLock;
use tracing::{error, info};

/// Telegram channel service
pub struct TelegramService {
    bot: Bot,
    persistence: Arc<RwLock<PersistenceService>>,
    provider: Arc<RwLock<ProviderService>>,
}

/// Bot commands
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "Start the bot")]
    Start,
    #[command(description = "Get help")]
    Help,
    #[command(description = "Clear conversation history")]
    Clear,
}

impl TelegramService {
    /// Create a new Telegram service
    pub fn new(token: &str, persistence: PersistenceService, provider: ProviderService) -> Self {
        let bot = Bot::new(token);
        info!("Telegram service initialized");
        Self {
            bot,
            persistence: Arc::new(RwLock::new(persistence)),
            provider: Arc::new(RwLock::new(provider)),
        }
    }

    /// Validate the bot token by making a test API call
    pub async fn validate_token(&self) -> Result<()> {
        info!("Validating Telegram bot token...");
        
        match self.bot.get_me().await {
            Ok(_me) => {
                info!("Telegram bot token is valid");
                Ok(())
            }
            Err(teloxide::RequestError::Api(teloxide::ApiError::InvalidToken)) => {
                Err(anyhow!(
                    "Invalid Telegram bot token. Please check TELEGRAM_BOT_TOKEN environment variable \
                    or edit ~/.rustclaw/rustclaw.toml"
                ))
            }
            Err(e) => {
                Err(anyhow!("Failed to validate Telegram bot token: {}", e))
            }
        }
    }

    /// Run the Telegram service (this is a blocking call)
    pub async fn run(self) -> Result<()> {
        // Validate token first
        self.validate_token().await?;

        info!("Starting Telegram bot...");

        let persistence = self.persistence.clone();
        let provider = self.provider.clone();

        // Use Dispatcher instead of repl for better error handling
        let handler = Update::filter_message()
            .branch(
                dptree::entry()
                    .filter_command::<Command>()
                    .endpoint(Self::handle_command)
            )
            .branch(
                dptree::filter(|msg: Message| msg.text().is_some())
                    .endpoint(Self::handle_message)
            );

        let mut dispatcher = Dispatcher::builder(self.bot.clone(), handler)
            .dependencies(dptree::deps![
                persistence,
                provider
            ])
            .error_handler(LoggingErrorHandler::with_custom_text(
                "An error has occurred in the dispatcher",
            ))
            .build();

        // Run with proper error handling
        dispatcher.dispatch().await;
        
        Ok(())
    }

    /// Handle bot commands
    async fn handle_command(
        bot: Bot,
        msg: Message,
        cmd: Command,
    ) -> Result<(), teloxide::RequestError> {
        let chat_id = msg.chat.id;
        
        match cmd {
            Command::Start => {
                bot.send_message(chat_id, "Welcome to RustClaw! I'm your AI assistant. Send me a message to start chatting.")
                    .await?;
            }
            Command::Help => {
                bot.send_message(chat_id, Command::descriptions().to_string())
                    .await?;
            }
            Command::Clear => {
                bot.send_message(chat_id, "Conversation history cleared.")
                    .await?;
            }
        }
        
        Ok(())
    }

    /// Handle regular messages
    async fn handle_message(
        bot: Bot,
        msg: Message,
        persistence: Arc<RwLock<PersistenceService>>,
        provider: Arc<RwLock<ProviderService>>,
    ) -> Result<(), teloxide::RequestError> {
        let text = match msg.text() {
            Some(t) => t,
            None => return Ok(()),
        };

        let chat_id = msg.chat.id;
        let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
        let user = User::new(user_id);

        // Handle regular message
        let rustclaw_msg = RustClawMessage::new(
            chat_id.0,
            user,
            MessageContent::Text(text.to_string()),
        );

        // Save message
        {
            let persistence = persistence.write().await;
            if let Err(e) = persistence.save_message(&rustclaw_msg).await {
                error!("Failed to save message: {}", e);
            }
        }

        // Get recent messages for context
        let recent_messages = {
            let persistence = persistence.read().await;
            persistence
                .get_recent_messages(chat_id.0, 10)
                .await
                .unwrap_or_default()
        };

        // Get AI response
        let response = {
            let provider = provider.read().await;
            provider.complete(&recent_messages, text).await
        };

        match response {
            Ok(response) => {
                bot.send_message(chat_id, &response).await?;
            }
            Err(e) => {
                error!("Failed to get AI response: {}", e);
                bot.send_message(chat_id, "Sorry, I encountered an error processing your request.")
                    .await?;
            }
        }

        Ok(())
    }
}
