# charm-local-llm

Rust CLI for Ollama local LLM DevOps — automates setup, optimization, and lifecycle management of Ollama on CachyOS with NVIDIA RTX 4090, with deep integration into Crushing TUI and Kilocode.

## What It Does

- Starts/stops Ollama with optimized settings for your platform
- Manages local models (pull, ensure, remove)
- Configures Qdrant vector database via docker-compose
- Warms up models for faster first inference
- **Generates Crush config** (`~/.config/crush/crush.json`) for local-first agentic coding
- **Patches Kilocode config** to use local Ollama for indexing/embeddings
- Generates `CRUSH.md` and `AGENTS.md` project context files
- Manages Ollama systemd service lifecycle

## Architecture

```text
charm-local-llm (this tool)
├── Ollama (local LLM inference, RTX 4090 optimized)
│   ├── qwen3-coder:30b-gpu    ← primary devops model
│   ├── devstral-small-2-gpu   ← quick responses
│   ├── gemma4:26b-devops      ← alternative
│   └── nomic-embed-text       ← embeddings (768 dims)
├── Qdrant (vector DB for semantic search)
├── Crush (TUI coding assistant → uses local Ollama)
│   └── ~/.config/crush/crush.json (auto-generated)
└── Kilocode (VS Code AI assistant)
    └── ~/.config/kilo/kilo.json (indexing patched for Ollama)
```

### Kilocode Integration

Kilocode routes chat models through the **Kilo Gateway** (`kilo/kilo-auto/free`), which does not support direct Ollama provider routing. However, this tool configures:

- **Indexing/embeddings**: Local Ollama + Qdrant for semantic code search
- **Project context**: `AGENTS.md` with model info that Kilocode reads

For chat models, the Kilo Gateway handles fallback between free and balanced tiers. The gateway is the only chat provider available in Kilocode's config.

### Crush Integration

Crush fully supports local Ollama as a provider. This tool generates:

- `~/.config/crush/crush.json` with Ollama as the primary provider
- `qwen3-coder:30b-gpu` mapped to large/medium model slots
- `devstral-small-2-gpu` mapped to the small slot
- `discover_models: true` for auto-detection of additional models
- `CRUSH.md` with project context for Crush to follow

## Prerequisites

- Rust (stable) with rustfmt and clippy
- Ollama installed and on PATH
- Optional: Docker + docker-compose (for Qdrant)
- Optional: `go` (for checkmake installation)

## Quick Start

```bash
make setup   # Install dependencies and verify tools
make build   # Compile
make start   # Start Ollama + models + Qdrant + generate Crush/Kilo config
make status  # Show environment status
```

After `make start`:

- Ollama is running with GPU-optimized settings
- Qdrant vector DB is running
- `~/.config/crush/crush.json` is generated for Crush TUI
- `CRUSH.md` and `AGENTS.md` are written to project root
- Kilocode indexing is patched for local Ollama

## CLI Commands

```bash
# Environment lifecycle
charm start              # Start everything + generate Crush/Kilo config
charm stop               # Stop everything
charm status             # Show status

# Crush integration
charm crush init         # Generate/update ~/.config/crush/crush.json
charm crush status       # Show Crush config status
charm crush context      # Generate CRUSH.md

# Kilocode integration
charm kilo init          # Patch ~/.config/kilo/kilo.json for local Ollama indexing
charm kilo status        # Show Kilo config status
charm kilo context       # Generate AGENTS.md

# Models
charm models list                              # List installed models
charm models ensure qwen3-coder:30b-gpu        # Ensure model exists
charm models remove old-model                   # Remove model

# Service management (systemd)
charm service start|stop|restart|status

# Qdrant vector DB
charm qdrant start|stop|status
```

## Make Targets

```bash
make setup             # Install deps, verify Rust/Ollama/Docker/checkmake
make build             # Compile in debug mode
make build-release     # Compile in release mode
make build-check       # Type-check without building
make test              # Run all tests
make test-unit         # Run unit tests only
make lint              # Run clippy + fmt check + checkmake
make fmt               # Auto-format code
make fix               # Auto-fix clippy warnings + format
make clean             # Remove build artifacts
make run ARGS="start"  # Run CLI with args
make ci                # Full CI pipeline (lint + test)
make pre-commit        # Pre-commit checks (format + lint + unit tests)
```

## Development

```bash
make setup       # First time setup
make build       # Build
make test        # Run tests (13 integration tests)
make lint        # Check code quality (clippy + fmt + checkmake)
make fix         # Auto-fix issues
make fmt-check   # Verify formatting
```

## License

MIT
