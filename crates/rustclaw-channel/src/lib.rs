use anyhow::Result;
use rustclaw_persistence::PersistenceService;
use rustclaw_provider::ProviderService;
use rustclaw_types::{Message as RustClawMessage, MessageContent, User};
use std::sync::Arc;
use teloxide::{prelude::*, utils::command::BotCommands};
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

    /// Run the Telegram service (this is a blocking call)
    pub async fn run(self) -> Result<()> {
        info!("Starting Telegram bot...");

        let persistence = self.persistence.clone();
        let provider = self.provider.clone();

        teloxide::repl(self.bot.clone(), move |bot: Bot, msg: teloxide::types::Message| {
            let persistence = persistence.clone();
            let provider = provider.clone();
            async move {
                if let Some(text) = msg.text() {
                    let chat_id = msg.chat.id;
                    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
                    let user = User::new(user_id);

                    // Handle commands
                    if let Ok(command) = Command::parse(text, "rustclaw_bot") {
                        match command {
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
                    } else {
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
                    }
                }
                respond(())
            }
        })
        .await;

        Ok(())
    }
}
