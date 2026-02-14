# RustClaw

A production-ready minimal multi-channel AI gateway written in Rust, inspired by [OpenClaw](https://github.com/openclaw/openclaw).

## Features

- **Telegram Integration**: Chat with your AI assistant via Telegram
- **Multiple LLM Providers**: Support for OpenAI and Ollama
- **SQLite Persistence**: Local-first conversation storage
- **Structured Logging**: journald/syslog support
- **Microservices Architecture**: Clean, maintainable codebase
- **Production Ready**: No "while true" loops - proper structured concurrency with streams

## Quick Start

### Prerequisites

- Rust 1.75 or higher
- SQLite3
- Telegram Bot Token (from [@BotFather](https://t.me/botfather))
- OpenAI API Key or Ollama running locally

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/rustclaw.git
cd rustclaw

# Build
cargo build --release

# Create config file
cp rustclaw.toml.example rustclaw.toml

# Set environment variables
export TELEGRAM_BOT_TOKEN="your_telegram_bot_token"
export OPENAI_API_KEY="your_openai_api_key"

# Run
./target/release/rustclaw-gateway
```

### Using Ollama

```bash
# Install and run Ollama
ollama serve

# Pull a model
ollama pull llama2

# Update config to use Ollama
# In rustclaw.toml:
# [providers]
# default = "ollama"
```

## Configuration

Configuration is loaded from `rustclaw.toml` and can be overridden with environment variables.

### Config File (`rustclaw.toml`)

```toml
[telegram]
bot_token = ""  # Overridden by TELEGRAM_BOT_TOKEN env var

[providers]
default = "openai"  # or "ollama"

[providers.openai]
api_key = ""  # Overridden by OPENAI_API_KEY env var
model = "gpt-5-mini"

[providers.ollama]
base_url = "http://localhost:11434"
model = "llama2"

[database]
path = "rustclaw.db"

[logging]
level = "info"  # trace, debug, info, warn, error
output = "journald"  # journald, syslog, stdout
```

### Environment Variables

- `TELEGRAM_BOT_TOKEN`: Telegram bot token
- `OPENAI_API_KEY`: OpenAI API key
- `OLLAMA_BASE_URL`: Ollama base URL (default: http://localhost:11434)
- `RUSTCLAW_LOG_LEVEL`: Log level (default: info)

## Architecture

RustClaw uses a service-oriented architecture with Tokio channels for communication:

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

## Development

```bash
# Run tests
cargo test

# Run clippy
cargo clippy

# Format code
cargo fmt

# Run in development mode
cargo run
```

## Deployment

### Systemd Service

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
Environment="TELEGRAM_BOT_TOKEN=your_token"
Environment="OPENAI_API_KEY=your_key"

[Install]
WantedBy=multi-user.target
```

### Docker

```bash
# Build image
docker build -t rustclaw .

# Run container
docker run -d \
  -e TELEGRAM_BOT_TOKEN="your_token" \
  -e OPENAI_API_KEY="your_key" \
  -v rustclaw-data:/data \
  rustclaw
```

## Roadmap

- [ ] Additional channels (Slack, Discord)
- [ ] Web UI for management
- [ ] Conversation export/import
- [ ] Multi-tenancy support
- [ ] Metrics and monitoring
- [ ] Hot configuration reload
- [ ] Plugin/skill system

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- Inspired by [OpenClaw](https://github.com/openclaw/openclaw)
- Built with Rust and Tokio
