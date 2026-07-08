# charm-local-llm

Rust CLI that automates setup, optimization, and lifecycle management of local Ollama LLMs on CachyOS RTX 4090 and Apple Silicon MacBooks. Generates coding assistant configs for Crush and Kilocode so your entire AI toolchain runs locally.

## Project Structure

```text
charm-local-llm/
├── src/
│   ├── main.rs                  Entry point
│   ├── lib.rs                   Module exports
│   ├── cli.rs                   clap CLI definitions (subcommands/args)
│   ├── commands.rs              Command implementations (start/stop/status/etc)
│   ├── config.rs                Config struct + platform-specific defaults
│   ├── crush.rs                 Crush config generation (~/.config/crush/crush.json)
│   ├── kilo_integration.rs      Kilo config patching + AGENTS.md generation
│   ├── modelfile.rs             Ollama modelfile parser
│   ├── ollama.rs                Ollama HTTP API client (models, warmup, create)
│   └── platform.rs              Platform detection, env loading, GPU checks
├── platform/
│   ├── cachyos-i9-32gb-nvidia-4090/   Linux + NVIDIA RTX 4090 (24GB VRAM)
│   ├── macos-m4-24gb/                 Apple Silicon M4 (24GB unified)
│   ├── macos-m4-32gb/                 Apple Silicon M4 (32GB unified)
│   ├── macos-m5-24gb/                 Apple Silicon M5 (24GB unified)
│   └── macos-m5-32gb/                 Apple Silicon M5 (32GB unified)
│       Each platform dir holds `.env` (env overrides, gitignored) and `.env.example`
│       (tracked template) and `modfiles/` (Ollama definitions).
│       Copy the example to a real `.env` for your platform before running:
│       `cp platform/<your-platform>/.env.example platform/<your-platform>/.env`
│       (the `.env` is gitignored so local overrides are never committed or lost on clone).
├── tests/
│   └── integration_test.rs
├── .crush/                      Crush TUI local data (DB, logs)
├── .kilo/                       Kilo project config (kilo.jsonc)
├── .vscode/settings.json
├── Cargo.toml
├── Makefile
├── docker-compose.yml           Qdrant vector DB container
├── AGENTS.md                    Auto-generated project context for Kilocode
├── CRUSH.md                     Auto-generated project context for Crush
├── LICENSE
└── README.md
```

## What It Does

- Starts/stops Ollama with GPU-optimized settings (KV cache, parallel requests, CUDA on Linux / Metal on macOS)
- Manages local models — pull, ensure from modelfile, remove, warmup
- Starts Qdrant vector database via docker-compose
- **Generates Crush config** (`~/.config/crush/crush.json`) for local-first agentic coding
- **Cleans up Kilocode config** (`~/.config/kilo/kilo.json`) by removing unsupported `indexing` blocks
- Generates `CRUSH.md` and `AGENTS.md` project context files
- Manages Ollama systemd service lifecycle

## Local Models (RTX 4090, 24GB VRAM)

| Model | Role | VRAM |
| ------- | ------ | ------ |
| `qwen3.6:27b-instruct-q4_K_M-gpu` | Primary — coding, complex reasoning | ~18GB |
| `qwen3:8b` | Lightweight general-purpose (newest Qwen3 arch) | ~5GB |
| `devstral-small-2-gpu` | Quick — fast responses, simple tasks | ~15GB |
| `nomic-embed-text` | Embeddings for semantic search (768 dims) | ~300MB |
| `gemma4:26b-devops` | Alternative coding model | ~17GB |

## Crush Integration

`kcharm start` generates `~/.config/crush/crush.json`:

- **Provider**: `ollama` at `http://localhost:11434/v1/` with `discover_models: true`
- **large + medium slots** → `qwen3.6:27b-instruct-q4_K_M-gpu` (8192 max tokens)
- **small slot** → `devstral-small-2-gpu` (4096 max tokens)
- **Context paths**: `CRUSH.md`, `AGENTS.md`, `.clinerules`
- **Permissions**: bash, view, edit, write, glob, grep

Also generates `CRUSH.md` in the project root with model info and guidelines for Crush to follow.

## Kilocode Integration

`kcharm start` (and `kcharm kilo init`) writes `AGENTS.md` in the project root with project context that Kilocode reads automatically, and patches `~/.config/kilo/kilo.json`:

- Registers an `Ollama Local (FREE)` provider pointing at the local Ollama endpoint (`http://localhost:11434/v1/`) with known model aliases (including the platform devops/quick models).
- Removes any unsupported `indexing` block.

Kilocode then runs chat/inference directly against local Ollama — no external gateway, so data stays on-machine.

## Quick Start

```bash
make setup        # Install deps, build, and install kcharm to ~/.local/bin
kcharm start      # Start Ollama + models + Qdrant + generate Crush/Kilo config
kcharm stop       # Stop everything
kcharm status     # Show environment status

# Or use cargo directly (no install needed):
cargo run -- start
cargo run -- status
```

## Installation

`make setup` builds and installs `kcharm` to `~/.local/bin/`. If `~/.local/bin` is not in your PATH:

- **Fish**: `set -U fish_user_paths ~/.local/bin $fish_user_paths`
- **Bash**: `echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc`
- **PowerShell**: `[Environment]::SetEnvironmentVariable('PATH', $env:PATH + ';$HOME\.local\bin', 'User')`

Or install manually:

```bash
cargo build
cp target/debug/kcharm ~/.local/bin/kcharm
```

## CLI Commands

```bash
kcharm start                          # Start everything + generate configs
kcharm stop                           # Stop everything
kcharm status                         # Show status

kcharm crush init                     # Generate ~/.config/crush/crush.json
kcharm crush status                   # Show Crush config status
kcharm crush context                  # Generate CRUSH.md

kcharm kilo init                      # Remove unsupported indexing block from kilo.json
kcharm kilo status                    # Show Kilo config status
kcharm kilo context                   # Generate AGENTS.md

kcharm models list                    # List installed models
kcharm models ensure qwen3.6:27b-instruct-q4_K_M-gpu  # Ensure model exists
kcharm models remove old-model        # Remove model

kcharm service start|stop|restart|status
kcharm qdrant start|stop|status
```

## Make Targets

```bash
make setup        # Install deps, build, and install kcharm to ~/.local/bin
make build        # Compile (debug)
make build-release # Compile (release)
make test         # Run all tests
make lint         # clippy + fmt + checkmake
make fix          # Auto-fix clippy + format
make ci           # Full CI pipeline (lint + test)
make clean        # Remove build artifacts
make run ARGS="<command>"  # Run CLI with args (e.g., make run ARGS="start")

# Installation targets
make install      # Build and install kcharm to ~/.local/bin
make setup-fish   # Install and add to fish PATH
make setup-powershell # Install for PowerShell

# Convenience targets (wraps 'cargo run -- <command>')
make run-start    # Start Ollama + models + Qdrant
make run-stop     # Stop everything
make run-status   # Show status
make run-models ARGS="list"   # Manage models
make run-qdrant ARGS="start"  # Manage Qdrant
make crush-init   # Generate Crush config
make crush-status # Show Crush config status
make kilo-init    # Remove unsupported indexing block from kilo.json
make kilo-status  # Show Kilo config status
```

## Prerequisites

- Rust (stable) with rustfmt and clippy
- Ollama installed and on PATH
- Optional: Docker + docker-compose (for Qdrant)

## License

MIT
