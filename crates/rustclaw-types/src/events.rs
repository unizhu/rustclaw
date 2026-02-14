use crate::{Message, ChatId};
use serde::{Deserialize, Serialize};

/// Events that can be sent between services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    /// New message received from a channel
    MessageReceived(Message),
    
    /// Response ready to send to a channel
    SendResponse {
        chat_id: ChatId,
        text: String,
    },
    
    /// Error occurred
    Error {
        service: String,
        message: String,
    },
    
    /// Service started
    ServiceStarted {
        service: String,
    },
    
    /// Service stopped
    ServiceStopped {
        service: String,
    },
}

/// Commands from gateway to services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    /// Shutdown all services
    Shutdown,
    
    /// Reload configuration
    ReloadConfig,
}
