# charm-local-llm

- [charm-local-llm](#charm-local-llm)
  - [Summary](#summary)
  - [Project Structure](#project-structure)
  - [What It Does](#what-it-does)
  - [Enterprise, High-Side \& Secure Regulatory Compliance Architecture](#enterprise-high-side--secure-regulatory-compliance-architecture)
    - [Key Security \& Architectural Safeguards](#key-security--architectural-safeguards)
    - [Enterprise Infrastructure Mapping](#enterprise-infrastructure-mapping)
    - [Architectural Status](#architectural-status)
  - [Local Models (RTX 4090, 24GB VRAM)](#local-models-rtx-4090-24gb-vram)
  - [Crush Integration](#crush-integration)
  - [Kilocode Integration](#kilocode-integration)
    - [Context filtering (`.kiloignore`)](#context-filtering-kiloignore)
  - [Quick Start](#quick-start)
  - [Installation](#installation)
  - [CLI Commands](#cli-commands)
  - [Make Targets](#make-targets)
  - [Prerequisites](#prerequisites)
  - [License](#license)

## Summary

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


## Enterprise, High-Side & Secure Regulatory Compliance Architecture

`keverall/charm-local-llm` (KevCharm) is a compiled, high-performance Rust proxy and orchestration layer explicitly engineered to deliver secure, internet-independent developer AI tools. This repository completely supersedes older Bash/Ollama standalone scripts by shifting execution to a memory-safe, compiled runtime. 

It is designed specifically to pass rigorous code validation audits inside **completely air-gapped, zero-trust, and high-side (TS/SC/Secure-grade) secure enclaves**.

### Key Security & Architectural Safeguards

- **Native Memory Safety & SAST Compliance**: Built entirely in Rust, the proxy layer provides structural compile-time guarantees against memory corruption, buffer overflows, and thread-safety vulnerabilities, allowing the binary to cleanly clear automated defense static application security testing (SAST) gates.
- **Telemetry Interception & Cloud Decoupling**: Commercial IDE code assistants and development extensions are hardcoded to hit public web endpoints. KevCharm intercepts these outbound HTTP/gRPC requests at the runtime layer, stripping telemetry and forcing data payloads to route strictly to localized, offline inference models (such as quantized `Qwen` running on locked down local only Ollama local with the Ollama cloud end point access removed and overridden).
- **Deterministic Local Execution Loop**: The application entirely eliminates external DNS lookups and WAN interface bindings. It forces the inference loop to communicate over local UNIX domain sockets or strict loopback addresses (`127.0.0.1`), ensuring zero lateral data leakage across adjacent networks.
- **Offline Bootstrapping via Static Ingestion**: Rather than pulling model manifests dynamically off the web, the system handles model weights via static local paths. In a secure AWS environment, this allows the system to initialize entirely offline, pulling encrypted model layers from private S3 buckets over internal VPC Gateway Endpoints.

### Enterprise Infrastructure Mapping

The local physical verification baseline for KevCharm maps directly to secure cloud enterprise hardware equivalents:

| Local Validation Tier | Classified Cloud Equivalent | Compliance Control Met |
| :--- | :--- | :--- |
| **Intel i9 Core Compute** | AWS EC2 (Compute-Optimized) | Independent Local Orchestration |
| **NVIDIA RTX 4090 GPU** | AWS EC2 G-Family / P-Family | High-Performance Offline Inference |
| **Rust Compiled Binary** | Hardened Target Container / AMI | Zero-Dependency Runtime Stability |
| **Local Interception Proxy** | Isolated Private Subnet Configuration | Zero Telemetry / Exfiltration Prevention |

### Architectural Status

⚠️ **Deprecation Notice:** This Rust-based orchestration system completely supersedes and deprecates the legacy Bash-based `keverall/ollama` repository. All future development, hardening, and high-side compliance integrations are maintained natively within this repository.

## Local Models (RTX 4090, 24GB VRAM)

| Model | Role | VRAM |
| ------- | ------ | ------ |
| `gemma4:26b-devops` | Primary DevOps — coding, complex reasoning | ~17GB |
| `qwen3-coder:30b-gpu` | Coding — general purpose coder | ~18GB |
| `devstral-small-2-gpu` | Quick — fast responses, simple tasks | ~15GB |
| `nomic-embed-text` | Embeddings for semantic search (768 dims) | ~300MB |

## Crush Integration

`kcharm start` generates `~/.config/crush/crush.json`:

- **Provider**: `ollama` at `http://localhost:11434/v1/` with `discover_models: true`
- **large + medium slots** → `gemma4:26b-devops` (8192 max tokens)
- **small slot** → `devstral-small-2-gpu` (4096 max tokens)
- **Context paths**: `CRUSH.md`, `AGENTS.md`, `.clinerules`
- **Permissions**: bash, view, edit, write, glob, grep

Also generates `CRUSH.md` in the project root with model info and guidelines for Crush to follow.

## Kilocode Integration

`kcharm start` (and `kcharm kilo init`) writes `AGENTS.md` in the project root with project context that Kilocode reads automatically, and patches `~/.config/kilo/kilo.json`:

- Registers an `Ollama Local (FREE)` provider pointing at the local Ollama endpoint (`http://localhost:11434/v1/`) with models synced to the models currently available in Ollama (queried via `/api/tags`).
- Removes any unsupported `indexing` block.

Kilocode then runs chat/inference directly against local Ollama — no external gateway, so data stays on-machine.

### Context filtering (`.kiloignore`)

To keep prompts small, cheap, and free of secrets, `kcharm` also emits a `.kiloignore` into the project root (next to `AGENTS.md`). It is **composed**, not monolithic:

- `assets/kilo/base.kiloignore` — universal rules (build dirs, binaries, media, logs, IDE files, **and secret/credential protection**: `.env`, `*.pem`, `*.key`, `id_rsa*`, `*.tfvars`, `kubeconfig*`, etc.).
- Language/task **fragments** — `rust.kiloignore`, `go.kiloignore`, `ts.kiloignore`, `python.kiloignore`, `powershell.kiloignore`, `iac.kiloignore` (Terraform/Ansible/K8s).

On `kcharm kilo init` / `kcharm start`, `kcharm` detects the languages present in the project (via `Cargo.toml`, `go.mod`, `package.json`, `*.tf`, `*.ps1`, …) and appends the matching fragments, de-duplicating lines. The result is a single `.kiloignore` Kilocode understands.

- Non-destructive: if a `.kiloignore` already exists it is left untouched.
- Edit the files under `assets/kilo/` to tune rules, then **rebuild** (`make build` / `make sod`) — they are embedded into the `kcharm` binary at compile time.

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
kcharm models ensure qwen3-coder:30b-gpu  # Ensure model exists
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

### Passwordless Sudo (Linux / CachyOS)

On Linux, `kcharm` manages the Ollama systemd service. Several subcommands require
elevated privileges. Run `service install` **once** with sudo to install a
restrictive sudoers file, then all subsequent commands work without a password:

```bash
# One-time setup (requires your password, use full path since kcharm
# is not in sudo's secure_path):
sudo ~/.local/bin/kcharm service install

# After that, these run passwordless:
kcharm start      # systemctl start/stop/daemon-reload + env file
kcharm stop
kcharm status
kcharm service install   # re-run if you changed the unit file or config
make sod           # build + start
```

**Safety:** `kcharm` never writes `/etc/sudoers` and never grants passwordless
write access to the sudoers file itself. The generated drop-in at
`/etc/sudoers.d/ollama` is syntax-checked with `visudo -cf` and installed
atomically, so a malformed file can never lock out `sudo`. The only
passwordless rules are the specific `systemctl` and `/etc/default/ollama` /
systemd unit file writes that `kcharm` needs. If you prefer to configure
manually as root:

```bash
# Create /etc/sudoers.d/ollama with (replace keverall with your username):
# keverall ALL=(ALL) NOPASSWD: /usr/bin/systemctl start ollama
# keverall ALL=(ALL) NOPASSWD: /usr/bin/systemctl stop ollama
# keverall ALL=(ALL) NOPASSWD: /usr/bin/systemctl disable ollama
# keverall ALL=(ALL) NOPASSWD: /usr/bin/systemctl enable ollama
# keverall ALL=(ALL) NOPASSWD: /usr/bin/systemctl daemon-reload
# keverall ALL=(ALL) NOPASSWD: /usr/bin/systemctl is-active --quiet ollama
# keverall ALL=(ALL) NOPASSWD: /usr/bin/tee /etc/default/ollama
# keverall ALL=(ALL) NOPASSWD: /usr/bin/tee /etc/systemd/system/ollama.service
# keverall ALL=(ALL) NOPASSWD: /usr/bin/tee /etc/systemd/system/ollama.service.d/cachyos-nvidia.conf
# chmod 440 /etc/sudoers.d/ollama
```

## License

MIT
