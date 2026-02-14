use crate::{Id, User};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Message ID type
pub type MessageId = Id<Message>;

/// Chat ID type (Telegram chat ID)
pub type ChatId = i64;

/// Message from any channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub chat_id: ChatId,
    pub sender: User,
    pub content: MessageContent,
    pub timestamp: DateTime<Utc>,
}

/// Content of a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    Text(String),
    // Future: Image, File, etc.
}

impl Message {
    pub fn new(chat_id: ChatId, sender: User, content: MessageContent) -> Self {
        Self {
            id: MessageId::new(),
            chat_id,
            sender,
            content,
            timestamp: Utc::now(),
        }
    }
}
