use anyhow::Result;
use rustclaw_types::{Message, User};
use sqlx::{Row, SqlitePool};
use tracing::info;

/// Persistence service for storing data in SQLite
pub struct PersistenceService {
    pool: SqlitePool,
}

impl PersistenceService {
    /// Create a new persistence service
    pub async fn new(database_path: &str) -> Result<Self> {
        let database_url = format!("sqlite:{}?mode=rwc", database_path);
        let pool = SqlitePool::connect(&database_url).await?;
        
        let service = Self { pool };
        service.run_migrations().await?;
        
        info!("Persistence service initialized with database: {}", database_path);
        Ok(service)
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                telegram_user_id INTEGER UNIQUE NOT NULL,
                username TEXT,
                first_name TEXT,
                last_name TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                chat_id INTEGER NOT NULL,
                user_id TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );

            CREATE INDEX IF NOT EXISTS idx_messages_chat_id ON messages(chat_id);
            CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);
            "#,
        )
        .execute(&self.pool)
        .await?;

        info!("Database migrations completed");
        Ok(())
    }

    /// Save a user to the database
    pub async fn save_user(&self, user: &User) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO users (id, telegram_user_id, username, first_name, last_name)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(user.id.to_string())
        .bind(user.telegram_user_id)
        .bind(&user.username)
        .bind(&user.first_name)
        .bind(&user.last_name)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Save a message to the database
    pub async fn save_message(&self, message: &Message) -> Result<()> {
        // First save the user
        self.save_user(&message.sender).await?;

        // Then save the message
        let content = match &message.content {
            rustclaw_types::MessageContent::Text(text) => text,
        };

        sqlx::query(
            r#"
            INSERT INTO messages (id, chat_id, user_id, content, timestamp)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(message.id.to_string())
        .bind(message.chat_id)
        .bind(message.sender.id.to_string())
        .bind(content)
        .bind(message.timestamp.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get recent messages for a chat
    pub async fn get_recent_messages(&self, chat_id: i64, limit: i32) -> Result<Vec<Message>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                m.id as message_id,
                m.chat_id,
                m.content,
                m.timestamp,
                u.id as user_id,
                u.telegram_user_id,
                u.username,
                u.first_name,
                u.last_name
            FROM messages m
            JOIN users u ON m.user_id = u.id
            WHERE m.chat_id = ?
            ORDER BY m.timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(chat_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let messages = rows
            .iter()
            .map(|row| {
                use rustclaw_types::{Id, MessageContent};
                Message {
                    id: Id::default(), // We'll parse from string
                    chat_id: row.get("chat_id"),
                    sender: User {
                        id: Id::default(),
                        telegram_user_id: row.get("telegram_user_id"),
                        username: row.get("username"),
                        first_name: row.get("first_name"),
                        last_name: row.get("last_name"),
                    },
                    content: MessageContent::Text(row.get("content")),
                    timestamp: chrono::DateTime::parse_from_rfc3339(row.get("timestamp"))
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                }
            })
            .collect();

        Ok(messages)
    }
}
