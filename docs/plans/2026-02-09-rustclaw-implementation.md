# RustClaw Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a production-ready minimal multi-channel AI gateway in Rust with Telegram integration, OpenAI + Ollama providers, SQLite persistence, and journald logging.

**Architecture:** Service-oriented architecture using Tokio channels for communication between microservices. Each service (Gateway, Channel, Provider, Persistence, Logging) owns its state and communicates via typed messages. No "while true" loops - proper structured concurrency with streams.

**Tech Stack:** Rust, Tokio, Teloxide (Telegram), async-openai (OpenAI + Ollama), sqlx (SQLite), tracing (structured logging), anyhow/thiserror (error handling)

---

## Phase 1: Foundation & Workspace Setup

### Task 1: Initialize Cargo Workspace

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `.gitignore`
- Create: `README.md`
- Create: `LICENSE`

**Implementation:**

1. Create workspace `Cargo.toml`:
```toml
[workspace]
members = [
    "crates/rustclaw-types",
    "crates/rustclaw-logging",
    "crates/rustclaw-persistence",
    "crates/rustclaw-provider",
    "crates/rustclaw-channel",
    "crates/rustclaw-gateway",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Uni Zhu <uni@example.com>"]
license = "MIT"

[workspace.dependencies]
tokio = { version = "1.42", features = ["full"] }
tokio-stream = "0.1"
tokio-util = { version = "0.7", features = ["rt"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
anyhow = "1.0"
thiserror = "2.0"
chrono = { version = "0.4", features = ["serde"] }
futures = "0.3"
config = "0.14"
dotenvy = "0.15"
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
teloxide = { version = "0.13", features = ["macros"] }
async-openai = "0.27"
uuid = { version = "1.11", features = ["v4", "serde"] }
```

2. Create `.gitignore`:
```
/target
/Cargo.lock
*.db
*.db-shm
*.db-wal
.env
rustclaw.toml
.DS_Store
*.log
```

3. Create README.md
4. Create LICENSE (MIT)
5. Create crates directory structure
6. Initialize git and commit

### Task 2: Implement Shared Types

**Files:**
- Create: `crates/rustclaw-types/Cargo.toml`
- Create: `crates/rustclaw-types/src/lib.rs`
- Create: `crates/rustclaw-types/src/message.rs`
- Create: `crates/rustclaw-types/src/user.rs`
- Create: `crates/rustclaw-types/src/provider.rs`
- Create: `crates/rustclaw-types/src/events.rs`

**Implementation:**

Define core types:
- `Message`, `MessageId`, `MessageContent`
- `User`, `UserId`, `UserPreferences`
- `Conversation`, `ConversationId`
- `Provider` enum
- Service event types

### Task 3: Implement Logging Service

**Files:**
- Create: `crates/rustclaw-logging/Cargo.toml`
- Create: `crates/rustclaw-logging/src/lib.rs`

**Implementation:**

Set up tracing subscriber with journald/syslog support.

### Task 4: Implement Persistence Service

**Files:**
- Create: `crates/rustclaw-persistence/Cargo.toml`
- Create: `crates/rustclaw-persistence/src/lib.rs`
- Create: `migrations/001_init.sql`

**Implementation:**

SQLite database with migrations, async API via channels.

### Task 5: Implement Provider Service

**Files:**
- Create: `crates/rustclaw-provider/Cargo.toml`
- Create: `crates/rustclaw-provider/src/lib.rs`
- Create: `crates/rustclaw-provider/src/traits.rs`
- Create: `crates/rustclaw-provider/src/openai.rs`
- Create: `crates/rustclaw-provider/src/ollama.rs`

**Implementation:**

Provider trait, OpenAI client, Ollama client.

### Task 6: Implement Channel Service

**Files:**
- Create: `crates/rustclaw-channel/Cargo.toml`
- Create: `crates/rustclaw-channel/src/lib.rs`
- Create: `crates/rustclaw-channel/src/traits.rs`
- Create: `crates/rustclaw-channel/src/telegram.rs`

**Implementation:**

Channel trait, Telegram implementation using teloxide.

### Task 7: Implement Gateway Service

**Files:**
- Create: `crates/rustclaw-gateway/Cargo.toml`
- Create: `crates/rustclaw-gateway/src/main.rs`
- Create: `crates/rustclaw-gateway/src/config.rs`
- Create: `crates/rustclaw-gateway/src/service.rs`

**Implementation:**

Main orchestrator that coordinates all services.

### Task 8: Create Configuration Files

**Files:**
- Create: `rustclaw.toml` (example)
- Create: `.env.example`

**Implementation:**

Default configuration file and environment variable examples.

### Task 9: Integration & Testing

**Files:**
- Create: `tests/integration_test.rs`

**Implementation:**

End-to-end tests, verify all services work together.

### Task 10: Documentation & Deployment

**Files:**
- Update: `README.md`
- Create: `docs/DEPLOYMENT.md`

**Implementation:**

Complete documentation, deployment guide, systemd service file.

---

## Success Criteria

- ✅ `cargo check` passes
- ✅ `cargo test` passes
- ✅ `cargo clippy` passes with no warnings
- ✅ `cargo build --release` succeeds
- ✅ Bot responds to Telegram messages
- ✅ OpenAI integration works
- ✅ Ollama integration works
- ✅ Conversations saved to SQLite
- ✅ Logs appear in journald
- ✅ Graceful shutdown works
- ✅ Config loads from TOML + env vars
