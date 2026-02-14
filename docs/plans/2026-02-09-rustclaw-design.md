# RustClaw Implementation Plan

**Date**: 2026-02-09  
**Project**: RustClaw - Production-ready minimal multi-channel AI gateway in Rust  
**Scope**: Telegram-only, OpenAI + Ollama providers, SQLite persistence, journald logging

## Overview

RustClaw is a Rust-based reimplementation of the core concepts from OpenClaw, a multi-channel AI assistant. This implementation focuses on production readiness, clean architecture, and maintainability.

### Goals
- Production-ready Telegram bot with LLM integration
- Clean microservices architecture using Tokio channels
- No "while true" loops - proper structured concurrency with streams
- Provider-agnostic design (OpenAI + Ollama initially)
- Local-first with SQLite persistence
- Structured logging to journald/syslog
- Easy configuration via TOML + environment variables

### Non-Goals
- Multiple messaging channels (Telegram only initially)
- Web UI or TUI
- Mobile apps
- Full feature parity with OpenClaw

## Architecture

### Service-Oriented Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Gateway Service                       │
│  (Orchestrator - manages lifecycle, routing, shutdown)  │
└────────┬─────────────────────────────────┬──────────────┘
         │                                 │
    ┌────▼────┐                      ┌────▼─────┐
    │ Channel │                      │ Provider  │
    │ Service │                      │  Service  │
    │(Telegram)│                     │(OpenAI+   │
    └────┬────┘                      │ Ollama)   │
         │                           └────┬─────┘
         │                                │
    ┌────▼────────────────────────────────▼─────┐
    │         Persistence Service (SQLite)       │
    │         Logging Service (journald)         │
    └────────────────────────────────────────────┘
```

### Key Principles
- Each service owns its state and resources
- Services communicate via typed messages over channels
- No shared mutable state between services
- Clean shutdown via `tokio_util::sync::CancellationToken`
- All services implement a common `Service` trait

### Channel Topology
- **Command Channel** (mpsc): Gateway → Services (start, stop, configure)
- **Event Channel** (broadcast): Services → All (message received, response ready, errors)
- **Request/Response** (oneshot): For synchronous queries (e.g., get conversation history)

## Services

### 1. Gateway Service (Main Orchestrator)
- **Role**: Coordinates all services, handles startup/shutdown
- **Owns**: Configuration, cancellation token, service handles
- **Responsibilities**:
  - Load config from TOML file + env vars
  - Spawn and supervise all services
  - Route messages between services
  - Handle graceful shutdown (SIGTERM, SIGINT)

### 2. Channel Service (Telegram Integration)
- **Role**: Interface with Telegram Bot API
- **Uses**: `teloxide` crate
- **Responsibilities**:
  - Connect to Telegram via long polling (stream-based)
  - Parse incoming messages into internal `Message` type
  - Send responses back to Telegram
  - Store conversation state in Persistence Service

### 3. Provider Service (LLM Clients)
- **Role**: Manage LLM provider connections
- **Uses**: `async-openai` crate (supports both OpenAI and Ollama)
- **Responsibilities**:
  - Route requests to correct provider (OpenAI or Ollama)
  - Handle provider switching based on config
  - Stream responses (if supported)
  - Track token usage

### 4. Persistence Service (SQLite)
- **Role**: All data storage
- **Uses**: `sqlx` or `rusqlite`
- **Responsibilities**:
  - Store conversations and messages
  - Store user preferences
  - Store channel state
  - Provide async API via channel requests

### 5. Logging Service (Journald/Syslog)
- **Role**: Structured logging
- **Uses**: `tracing` + `tracing-journald` or `tracing-subscriber`
- **Responsibilities**:
  - Log all service activities
  - Structured logging with service context
  - Support log levels (INFO, WARN, ERROR)

## Data Models

### Core Types

```rust
// Message from any channel
struct Message {
    id: MessageId,
    channel: Channel,
    sender: User,
    content: MessageContent,
    timestamp: DateTime<Utc>,
    conversation_id: ConversationId,
}

// User across all channels
struct User {
    id: UserId,
    channel_user_id: String,  // Telegram user ID
    username: Option<String>,
    preferences: UserPreferences,
}

// Conversation context
struct Conversation {
    id: ConversationId,
    user_id: UserId,
    messages: Vec<Message>,
    provider: Provider,  // Which LLM to use
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

// Provider selection
enum Provider {
    OpenAI { model: String },
    Ollama { model: String, base_url: String },
}
```

### Service Messages (Channel Communication)

```rust
enum GatewayCommand {
    Shutdown,
    ReloadConfig,
}

enum ChannelEvent {
    MessageReceived(Message),
    SendResponse { chat_id: ChatId, text: String },
}

enum ProviderRequest {
    Complete { conversation: Conversation, prompt: String },
    Response(Response),
}

enum PersistenceRequest {
    SaveMessage(Message),
    GetConversation(ConversationId, oneshot::Sender<Option<Conversation>>),
}
```

## Configuration

### Config Structure (`rustclaw.toml`)

```toml
[telegram]
bot_token = ""  # Can be overridden via TELEGRAM_BOT_TOKEN env var

[providers]
default = "openai"  # or "ollama"

[providers.openai]
api_key = ""  # Overridden via OPENAI_API_KEY env var
model = "gpt-4-turbo-preview"

[providers.ollama]
base_url = "http://localhost:11434"
model = "llama2"

[database]
path = "rustclaw.db"  # SQLite file location

[logging]
level = "info"  # trace, debug, info, warn, error
output = "journald"  # journald, syslog, stdout
```

### Environment Variable Overrides
- `TELEGRAM_BOT_TOKEN` → `telegram.bot_token`
- `OPENAI_API_KEY` → `providers.openai.api_key`
- `OLLAMA_BASE_URL` → `providers.ollama.base_url`
- `RUSTCLAW_LOG_LEVEL` → `logging.level`

### Config Loading
1. Load from `rustclaw.toml` (or `~/.config/rustclaw/config.toml`)
2. Override with env vars
3. Validate on startup (required fields present)
4. Hot reload via `GatewayCommand::ReloadConfig`

## Error Handling

### Error Strategy
- Use `anyhow` for application errors (simple error propagation)
- Use `thiserror` for library errors (custom error types)
- Each service has its own error type (e.g., `ChannelError`, `ProviderError`)
- Errors are logged with full context via `tracing`
- Fatal errors trigger graceful shutdown
- Transient errors are retried with exponential backoff

### Error Categories

```rust
#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("Telegram API error: {0}")]
    Telegram(#[from] teloxide::ApiError),
    
    #[error("Rate limited, retrying in {0}s")]
    RateLimited(u64),
    
    #[error("Network error: {0}")]
    Network(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("OpenAI API error: {0}")]
    OpenAI(String),
    
    #[error("Ollama connection failed: {0}")]
    OllamaConnection(String),
    
    #[error("Model not available: {0}")]
    ModelNotAvailable(String),
}
```

### Retry Logic
- Use `tokio-retry` crate
- Exponential backoff for network errors
- Max 3 retries before giving up
- Log retry attempts

## Project Structure

### Workspace Layout

```
rustclaw/
├── Cargo.toml              # Workspace config
├── rustclaw.toml           # Default config file
├── .env.example            # Example environment variables
├── README.md
├── LICENSE
│
├── crates/
│   ├── rustclaw-gateway/      # Gateway service (main entry point)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── service.rs
│   │       └── config.rs
│   │
│   ├── rustclaw-channel/      # Channel trait + Telegram impl
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs      # Channel trait
│   │       └── telegram.rs    # Telegram implementation
│   │
│   ├── rustclaw-provider/     # Provider trait + OpenAI/Ollama
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs      # Provider trait
│   │       ├── openai.rs
│   │       └── ollama.rs
│   │
│   ├── rustclaw-persistence/  # SQLite persistence
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── sqlite.rs
│   │
│   ├── rustclaw-logging/      # Logging service
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   │
│   └── rustclaw-types/        # Shared types
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs         # Message, User, Conversation, etc.
│
├── migrations/                # SQLite migrations
│   └── 001_init.sql
│
└── tests/                     # Integration tests
    └── integration_test.rs
```

### Dependencies Strategy
- **Common**: `tokio`, `tracing`, `anyhow`, `thiserror`, `serde`
- **Gateway**: `config`, `dotenvy`, `signal-hook`
- **Channel**: `teloxide`, `tokio-stream`
- **Provider**: `async-openai`
- **Persistence**: `sqlx` (async)
- **Logging**: `tracing-subscriber`, `tracing-journald`

## Implementation Phases

### Phase 1: Foundation
- Set up workspace and crates
- Implement shared types
- Set up logging service
- Configuration loading

### Phase 2: Core Services
- Gateway service skeleton
- Persistence service with SQLite
- Database migrations

### Phase 3: Channel & Providers
- Channel trait
- Telegram implementation
- Provider trait
- OpenAI implementation
- Ollama implementation

### Phase 4: Integration
- Wire all services together
- Message routing
- Error handling
- Graceful shutdown

### Phase 5: Testing & Polish
- Unit tests
- Integration tests
- Documentation
- README and examples

## Testing Strategy

### Unit Tests
- Each crate has its own tests
- Mock services using trait implementations
- Test error handling paths

### Integration Tests
- End-to-end flow: Telegram → Provider → Response
- Database operations
- Configuration loading

### Manual Testing
- Run against real Telegram bot
- Test with both OpenAI and Ollama
- Verify logging output in journald

## Deployment

### Build
```bash
cargo build --release
```

### Run
```bash
./target/release/rustclaw-gateway
```

### Environment Setup
```bash
export TELEGRAM_BOT_TOKEN="your_token"
export OPENAI_API_KEY="your_key"
export RUSTCLAW_LOG_LEVEL="info"
```

### Systemd Service (Optional)
```ini
[Unit]
Description=RustClaw AI Assistant
After=network.target

[Service]
Type=simple
User=rustclaw
WorkingDirectory=/opt/rustclaw
ExecStart=/opt/rustclaw/rustclaw-gateway
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

## Success Criteria

- ✅ Compiles without warnings (`cargo check`)
- ✅ Passes all tests (`cargo test`)
- ✅ No clippy warnings (`cargo clippy`)
- ✅ Builds release binary successfully
- ✅ Telegram bot responds to messages
- ✅ OpenAI integration works
- ✅ Ollama integration works
- ✅ Conversations persisted to SQLite
- ✅ Logs appear in journald
- ✅ Graceful shutdown on SIGTERM/SIGINT
- ✅ Configuration loads from TOML + env vars

## Future Enhancements (Post-MVP)

- Additional channels (Slack, Discord)
- Web UI for management
- Conversation export/import
- Multi-tenancy support
- Metrics and monitoring
- Hot configuration reload
- Plugin/skill system
