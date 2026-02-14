use crate::Id;
use serde::{Deserialize, Serialize};

/// User ID type
pub type UserId = Id<User>;

/// User across all channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub telegram_user_id: i64,  // Telegram user ID
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

impl User {
    pub fn new(telegram_user_id: i64) -> Self {
        Self {
            id: UserId::new(),
            telegram_user_id,
            username: None,
            first_name: None,
            last_name: None,
        }
    }

    pub fn full_name(&self) -> String {
        match (&self.first_name, &self.last_name) {
            (Some(first), Some(last)) => format!("{} {}", first, last),
            (Some(first), None) => first.clone(),
            (None, Some(last)) => last.clone(),
            (None, None) => self.username.clone().unwrap_or_else(|| "Unknown".to_string()),
        }
    }
}
