# RustClaw

A production-ready minimal multi-channel AI gateway written in Rust, inspired by [OpenClaw](https://github.com/openclaw/openclaw).

## Features

- **Telegram Integration**: Chat with your AI assistant via Telegram
- **Multiple LLM Providers**: Support for OpenAI and Ollama
- **MCP Integration**: Connect to Model Context Protocol servers for extended tool capabilities
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

# Set environment variables (or edit ~/.rustclaw/rustclaw.toml after first run)
export TELEGRAM_BOT_TOKEN="your_telegram_bot_token"
export OPENAI_API_KEY="your_openai_api_key"

# Run (will auto-create ~/.rustclaw/rustclaw.toml on first run)
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

Configuration uses a layered approach with the following priority (highest to lowest):

1. **Environment variables** - Convenience vars like `TELEGRAM_BOT_TOKEN`, `OPENAI_API_KEY`
2. **Local config** - `./rustclaw.toml` in the current working directory (for project-specific overrides)
3. **Global config** - `~/.rustclaw/rustclaw.toml` (auto-created on first run if missing)

### Global Config (`~/.rustclaw/rustclaw.toml`)

This is the primary configuration file, automatically created on first run:

```toml
[telegram]
bot_token = ""  # Set via TELEGRAM_BOT_TOKEN env var

[providers]
default = "openai"  # or "ollama"

[providers.openai]
api_key = ""  # Set via OPENAI_API_KEY env var
model = "gpt-4o-mini"
base_url = ""  # Optional: Set via OPENAI_BASE_URL env var

[providers.ollama]
base_url = "http://localhost:11434"
model = "llama3"

[database]
path = "rustclaw.db"

[logging]
level = "info"  # trace, debug, info, warn, error
```

### Local Override (`./rustclaw.toml`)

Place a `rustclaw.toml` in your working directory to override specific settings for that project. For example:

```toml
[providers]
default = "ollama"  # Use Ollama for this project

[providers.ollama]
model = "codellama"  # Use a different model
```

### Environment Variables

| Variable | Description | Config Path |
|----------|-------------|-------------|
| `TELEGRAM_BOT_TOKEN` | Telegram bot token | `telegram.bot_token` |
| `OPENAI_API_KEY` | OpenAI API key | `providers.openai.api_key` |
| `OPENAI_BASE_URL` | OpenAI base URL | `providers.openai.base_url` |
| `OLLAMA_BASE_URL` | Ollama base URL | `providers.ollama.base_url` |
| `RUSTCLAW__*` | Any config value | Uses `__` as separator |

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

## Tool Calling

RustClaw supports OpenAI-compatible function calling (tool calling). You can register custom tools that the model can call:

### Defining a Tool

```rust
use rustclaw_provider::{ToolFunction, ToolRegistry, ProviderService};
use rustclaw_types::Tool;
use anyhow::Result;

// Define a custom tool
pub struct WeatherTool;

impl ToolFunction for WeatherTool {
    fn definition(&self) -> Tool {
        Tool::function(
            "get_weather",
            "Get current weather for a location",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "City and country, e.g. 'Paris, France'"
                    }
                },
                "required": ["location"],
                "additionalProperties": false
            }),
        )
    }

    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let location = args["location"].as_str().unwrap();
        // Your weather API logic here
        Ok(serde_json::json!({
            "location": location,
            "temperature": "22°C",
            "condition": "sunny"
        }))
    }
}

// Register tools
let mut registry = ToolRegistry::new();
registry.register(Box::new(WeatherTool));

// Create provider with tools
let service = ProviderService::with_tools(provider, registry);

// Use agentic loop for automatic tool execution
let response = service.complete_agentic(&messages, "What's the weather in Paris?", 5).await?;
```

### Built-in Tools

- `EchoTool` - Simple echo for testing
- `CurrentTimeTool` - Get current date/time

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

- [x] OpenAI-compatible tool calling support
- [x] MCP (Model Context Protocol) client support
- [ ] Additional channels (Slack, Discord)
- [ ] Web UI for management
- [ ] Conversation export/import
- [ ] Multi-tenancy support
- [ ] Metrics and monitoring
- [ ] Hot configuration reload
- [ ] Plugin/skill system with dynamic loading

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- Inspired by [OpenClaw](https://github.com/openclaw/openclaw)
- Built with Rust and Tokio
