use anyhow::{anyhow, Result};
use rustclaw_persistence::PersistenceService;
use rustclaw_provider::{CurrentTimeTool, EchoTool, ProviderService, ToolFunction, ToolRegistry};
use rustclaw_types::{Message as RustClawMessage, MessageContent, Tool, User};
use std::sync::Arc;
use teloxide::{error_handlers::LoggingErrorHandler, prelude::*, utils::command::BotCommands};
use tokio::sync::RwLock;
use tracing::{error, info};

/// Maximum message length for Telegram (4096 chars, but we use less to be safe)
const MAX_MESSAGE_LENGTH: usize = 4000;

/// Sensitive file patterns that require user confirmation
const SENSITIVE_PATTERNS: &[&str] = &[
    ".ssh/", "id_rsa", "id_ed25519", ".pem", ".key",
    ".pgp", ".gnupg", "credentials", "secrets", ".env",
    "password", "token", "api_key", "apikey",
    ".aws/", ".kube/", ".docker/",
];

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
    #[command(description = "Show available tools")]
    Tools,
}

impl TelegramService {
    /// Create a new Telegram service with default tools
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
            Err(teloxide::RequestError::Api(teloxide::ApiError::InvalidToken)) => Err(anyhow!(
                "Invalid Telegram bot token. Please check TELEGRAM_BOT_TOKEN environment variable \
                or edit ~/.rustclaw/rustclaw.toml"
            )),
            Err(e) => Err(anyhow!("Failed to validate Telegram bot token: {}", e)),
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
                    .endpoint(Self::handle_command),
            )
            .branch(
                dptree::filter(|msg: Message| msg.text().is_some())
                    .endpoint(Self::handle_message),
            );

        let mut dispatcher = Dispatcher::builder(self.bot.clone(), handler)
            .dependencies(dptree::deps![persistence, provider])
            .error_handler(LoggingErrorHandler::with_custom_text(
                "An error has occurred in the dispatcher",
            ))
            .build();

        // Run with proper error handling
        dispatcher.dispatch().await;

        Ok(())
    }

    /// Split a message into chunks that fit Telegram's limits
    fn split_message(text: &str) -> Vec<String> {
        if text.len() <= MAX_MESSAGE_LENGTH {
            return vec![text.to_string()];
        }

        let mut chunks = Vec::new();
        let mut current_chunk = String::new();

        // Try to split on paragraph breaks first, then sentences, then words
        for paragraph in text.split("\n\n") {
            if current_chunk.len() + paragraph.len() + 2 > MAX_MESSAGE_LENGTH {
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk.trim().to_string());
                    current_chunk = String::new();
                }

                // If paragraph itself is too long, split by sentences
                if paragraph.len() > MAX_MESSAGE_LENGTH {
                    for sentence in paragraph.split(". ") {
                        if current_chunk.len() + sentence.len() + 2 > MAX_MESSAGE_LENGTH {
                            if !current_chunk.is_empty() {
                                chunks.push(current_chunk.trim().to_string());
                                current_chunk = String::new();
                            }

                            // If sentence is too long, split by words
                            if sentence.len() > MAX_MESSAGE_LENGTH {
                                for word in sentence.split_whitespace() {
                                    if current_chunk.len() + word.len() + 1 > MAX_MESSAGE_LENGTH {
                                        if !current_chunk.is_empty() {
                                            chunks.push(current_chunk.trim().to_string());
                                        }
                                        current_chunk = word.to_string();
                                    } else {
                                        if !current_chunk.is_empty() {
                                            current_chunk.push(' ');
                                        }
                                        current_chunk.push_str(word);
                                    }
                                }
                            } else {
                                current_chunk = sentence.to_string();
                            }
                        } else {
                            if !current_chunk.is_empty() {
                                current_chunk.push_str(". ");
                            }
                            current_chunk.push_str(sentence);
                        }
                    }
                } else {
                    current_chunk = paragraph.to_string();
                }
            } else {
                if !current_chunk.is_empty() {
                    current_chunk.push_str("\n\n");
                }
                current_chunk.push_str(paragraph);
            }
        }

        if !current_chunk.trim().is_empty() {
            chunks.push(current_chunk.trim().to_string());
        }

        chunks
    }

    /// Send a message, splitting if necessary
    async fn send_message_safe(
        bot: &Bot,
        chat_id: ChatId,
        text: &str,
    ) -> Result<(), teloxide::RequestError> {
        let chunks = Self::split_message(text);
        for (i, chunk) in chunks.iter().enumerate() {
            if chunks.len() > 1 {
                bot.send_message(chat_id, format!("({}/{})\n\n{}", i + 1, chunks.len(), chunk))
                    .await?;
            } else {
                bot.send_message(chat_id, chunk).await?;
            }
        }
        Ok(())
    }

    /// Handle bot commands
    async fn handle_command(bot: Bot, msg: Message, cmd: Command) -> Result<(), teloxide::RequestError> {
        let chat_id = msg.chat.id;

        match cmd {
            Command::Start => {
                Self::send_message_safe(
                    &bot,
                    chat_id,
                    "🦀 Welcome to RustClaw!\n\nI'm your AI assistant powered by Rust. \
                     Send me a message to start chatting.\n\n\
                     /help - Show commands\n/tools - Show available tools",
                )
                .await?;
            }
            Command::Help => {
                Self::send_message_safe(&bot, chat_id, &Command::descriptions().to_string()).await?;
            }
            Command::Clear => {
                Self::send_message_safe(&bot, chat_id, "🗑️ Conversation history cleared.").await?;
            }
            Command::Tools => {
                Self::send_message_safe(
                    &bot,
                    chat_id,
                    "🔧 Available tools:\n\n\
                     📁 **bash** - Execute bash commands (ls, cat, grep, curl, git, etc.)\n\
                     📄 **read_file** - Read file contents\n\
                     📂 **list_dir** - List directory contents\n\
                     ⏰ **get_current_time** - Get current date/time\n\
                     📢 **echo** - Echo back a message\n\n\
                     ⚠️ Sensitive files (SSH keys, passwords) require your confirmation.",
                )
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
        let rustclaw_msg =
            RustClawMessage::new(chat_id.0, user, MessageContent::Text(text.to_string()));

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

        // Get AI response using agentic loop (handles tools automatically)
        let response = {
            let provider = provider.read().await;
            // Use agentic completion with configured max iterations
            provider.complete_agentic_default(&recent_messages, text).await
        };

        match response {
            Ok(response) => {
                Self::send_message_safe(&bot, chat_id, &response).await?;
            }
            Err(e) => {
                error!("Failed to get AI response: {}", e);
                Self::send_message_safe(
                    &bot,
                    chat_id,
                    &format!("❌ Error: {}", e),
                )
                .await?;
            }
        }

        Ok(())
    }
}

// ============================================================================
// System Tools for Bash Commands
// ============================================================================

/// Tool for executing bash commands (safe subset)
pub struct BashTool;

impl ToolFunction for BashTool {
    fn definition(&self) -> Tool {
        Tool::function(
            "bash",
            "Execute bash/shell commands on the system.\n\n\
             \n**SUPPORTED COMMANDS:**\n\
             - File ops: ls, cat, head, tail, find, grep, wc, tree, mkdir, cp, mv, touch\n\
             - System info: uname, date, whoami, pwd, df, du, free, ps, top, uptime\n\
             - Text processing: sed, awk, sort, uniq, cut, tr, jq\n\
             - Network: curl, wget, ping, nslookup, dig, nc (read-only)\n\
             - Archives: tar, zip, unzip, gzip\n\
             - Git: git status, git log, git diff, git branch, git show\n\
             - Package info: npm list, pip list, pip freeze, cargo tree, go list\n\
             - Misc: which, whereis, file, stat, chmod, chown (non-destructive)\n\
             \n**BLOCKED COMMANDS:**\n\
             - sudo, su (no privilege escalation)\n\
             - rm -rf /, mkfs, dd (dangerous disk operations)\n\
             - Fork bombs or infinite loops\n\
             \n**IMPORTANT:**\n\
             - For DELETING files (rm, rmdir), ask user for confirmation first!\n\
             - For READING sensitive files (SSH keys, .pem, .key, passwords, .env, credentials), ALWAYS ask user permission first!\n\
             - Set confirm_destructive=true only after user confirms",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The bash command to execute"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Timeout in seconds (default: 30, max: 120)",
                        "default": 30
                    },
                    "confirm_destructive": {
                        "type": "boolean",
                        "description": "Set to true if user confirmed destructive operations (rm, del, format)",
                        "default": false
                    },
                    "confirm_sensitive": {
                        "type": "boolean", 
                        "description": "Set to true if user confirmed reading sensitive files (keys, passwords, secrets)",
                        "default": false
                    }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        )
    }

    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let command = args
            .get("command")
            .and_then(|c| c.as_str())
            .ok_or_else(|| anyhow!("Missing 'command' argument"))?;

        let _timeout = args
            .get("timeout")
            .and_then(|t| t.as_u64())
            .unwrap_or(30)
            .min(120);

        let confirm_destructive = args
            .get("confirm_destructive")
            .and_then(|c| c.as_bool())
            .unwrap_or(false);

        let confirm_sensitive = args
            .get("confirm_sensitive")
            .and_then(|c| c.as_bool())
            .unwrap_or(false);

        // Block always-dangerous commands
        let dangerous = ["rm -rf /", "sudo ", "sudo\t", "mkfs", "dd if=", "> /dev/sd", ":(){ :|:& };:"];
        for pattern in dangerous {
            if command.contains(pattern) {
                return Ok(serde_json::json!({
                    "success": false,
                    "blocked": true,
                    "error": format!("Command blocked: contains unsafe pattern '{}'", pattern.trim())
                }));
            }
        }

        // Check for sensitive file access without confirmation
        if !confirm_sensitive {
            for pattern in SENSITIVE_PATTERNS {
                if command.contains(pattern) {
                    return Ok(serde_json::json!({
                        "success": false,
                        "needs_confirmation": true,
                        "confirmation_type": "sensitive_file",
                        "error": format!(
                            "⚠️ SENSITIVE FILE DETECTED: The command appears to access '{}' which may contain secrets, keys, or credentials.\n\nPlease ask the user: \"This command may access sensitive files. Do you want me to proceed?\"",
                            pattern
                        )
                    }));
                }
            }
        }

        // Check for destructive commands without confirmation
        if !confirm_destructive {
            let destructive_patterns = ["rm ", "rm -", "rmdir", "del ", "format ", "shred "];
            for pattern in destructive_patterns {
                if command.contains(pattern) {
                    return Ok(serde_json::json!({
                        "success": false,
                        "needs_confirmation": true,
                        "confirmation_type": "destructive",
                        "error": format!(
                            "⚠️ DESTRUCTIVE COMMAND: '{}'\n\nThis will delete files. Please ask the user: \"This command will delete files. Are you sure you want to proceed?\"",
                            command
                        )
                    }));
                }
            }
        }

        // Execute the command
        let output = std::process::Command::new("bash")
            .arg("-c")
            .arg(command)
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let success = output.status.success();

                // Truncate very long output
                let stdout_str = if stdout.len() > 15000 {
                    format!(
                        "{}...\n\n[Output truncated: showing first 15KB of {} bytes total]",
                        &stdout[..15000],
                        stdout.len()
                    )
                } else {
                    stdout.to_string()
                };

                Ok(serde_json::json!({
                    "success": success,
                    "stdout": stdout_str,
                    "stderr": stderr,
                    "exit_code": output.status.code()
                }))
            }
            Err(e) => Ok(serde_json::json!({
                "success": false,
                "error": format!("Failed to execute command: {}", e)
            })),
        }
    }
}

/// Tool for reading files (with sensitive file protection)
pub struct ReadFileTool;

impl ToolFunction for ReadFileTool {
    fn definition(&self) -> Tool {
        Tool::function(
            "read_file",
            "Read the contents of a file.\n\n\
             ⚠️ IMPORTANT: For sensitive files (SSH keys: id_rsa, id_ed25519; certificates: .pem, .key; \
             secrets: .env, credentials, passwords, tokens), ALWAYS ask the user for permission first!\n\
             Set confirm_sensitive=true only after user confirms.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file to read"
                    },
                    "lines": {
                        "type": "integer",
                        "description": "Maximum number of lines to read (default: 100)",
                        "default": 100
                    },
                    "confirm_sensitive": {
                        "type": "boolean",
                        "description": "Set to true if user confirmed reading sensitive files",
                        "default": false
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        )
    }

    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let path = args
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| anyhow!("Missing 'path' argument"))?;

        let max_lines = args
            .get("lines")
            .and_then(|l| l.as_u64())
            .unwrap_or(100) as usize;

        let confirm_sensitive = args
            .get("confirm_sensitive")
            .and_then(|c| c.as_bool())
            .unwrap_or(false);

        // Check for sensitive file access
        if !confirm_sensitive {
            let lower_path = path.to_lowercase();
            for pattern in SENSITIVE_PATTERNS {
                if lower_path.contains(&pattern.to_lowercase()) {
                    return Ok(serde_json::json!({
                        "success": false,
                        "needs_confirmation": true,
                        "confirmation_type": "sensitive_file",
                        "error": format!(
                            "⚠️ SENSITIVE FILE: '{}' appears to be a sensitive file (key, credential, or secret).\n\nPlease ask the user: \"This file may contain sensitive information. Do you want me to read it?\"",
                            path
                        )
                    }));
                }
            }
        }

        let content = std::fs::read_to_string(path);

        match content {
            Ok(content) => {
                let total_lines = content.lines().count();
                let lines: Vec<&str> = content.lines().take(max_lines).collect();
                Ok(serde_json::json!({
                    "success": true,
                    "content": lines.join("\n"),
                    "lines_read": lines.len(),
                    "total_lines": total_lines,
                    "truncated": total_lines > max_lines
                }))
            }
            Err(e) => Ok(serde_json::json!({
                "success": false,
                "error": format!("Failed to read file: {}", e)
            })),
        }
    }
}

/// Tool for listing directories
pub struct ListDirTool;

impl ToolFunction for ListDirTool {
    fn definition(&self) -> Tool {
        Tool::function(
            "list_dir",
            "List contents of a directory. Shows files and subdirectories with their types.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The directory path to list (default: current directory)"
                    }
                },
                "required": [],
                "additionalProperties": false
            }),
        )
    }

    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let path = args
            .get("path")
            .and_then(|p| p.as_str())
            .unwrap_or(".");

        let entries = std::fs::read_dir(path);

        match entries {
            Ok(entries) => {
                let mut files = Vec::new();
                let mut dirs = Vec::new();

                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        dirs.push(name);
                    } else {
                        files.push(name);
                    }
                }

                dirs.sort();
                files.sort();

                Ok(serde_json::json!({
                    "success": true,
                    "path": path,
                    "directories": dirs,
                    "files": files,
                    "total_dirs": dirs.len(),
                    "total_files": files.len(),
                    "total": dirs.len() + files.len()
                }))
            }
            Err(e) => Ok(serde_json::json!({
                "success": false,
                "error": format!("Failed to list directory: {}", e)
            })),
        }
    }
}

/// Tool for writing files
pub struct WriteFileTool;

impl ToolFunction for WriteFileTool {
    fn definition(&self) -> Tool {
        Tool::function(
            "write_file",
            "Write content to a file. Creates the file if it doesn't exist, overwrites if it does.\n\n\
             ⚠️ IMPORTANT: This will OVERWRITE existing files. Ask user confirmation before overwriting important files!",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content to write to the file"
                    },
                    "confirm_overwrite": {
                        "type": "boolean",
                        "description": "Set to true if user confirmed overwriting an existing file",
                        "default": false
                    }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
        )
    }

    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let path = args
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| anyhow!("Missing 'path' argument"))?;

        let content = args
            .get("content")
            .and_then(|c| c.as_str())
            .ok_or_else(|| anyhow!("Missing 'content' argument"))?;

        let confirm_overwrite = args
            .get("confirm_overwrite")
            .and_then(|c| c.as_bool())
            .unwrap_or(false);

        // Check if file exists
        if std::path::Path::new(path).exists() && !confirm_overwrite {
            return Ok(serde_json::json!({
                "success": false,
                "needs_confirmation": true,
                "confirmation_type": "overwrite",
                "error": format!(
                    "⚠️ FILE EXISTS: '{}' already exists. Overwriting will destroy its current contents.\n\nPlease ask the user: \"This file already exists. Do you want to overwrite it?\"",
                    path
                )
            }));
        }

        match std::fs::write(path, content) {
            Ok(_) => Ok(serde_json::json!({
                "success": true,
                "message": format!("Successfully wrote to '{}'", path)
            })),
            Err(e) => Ok(serde_json::json!({
                "success": false,
                "error": format!("Failed to write file: {}", e)
            })),
        }
    }
}

/// Create a default tool registry with common tools
pub fn create_default_tools() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(EchoTool));
    registry.register(Box::new(CurrentTimeTool));
    registry.register(Box::new(BashTool));
    registry.register(Box::new(ReadFileTool));
    registry.register(Box::new(ListDirTool));
    registry.register(Box::new(WriteFileTool));
    registry
}
