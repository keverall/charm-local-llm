# charm-local-llm

Rust CLI that automates setup, optimization, and lifecycle management of local Ollama LLMs on CachyOS RTX 4090 and Apple Silicon MacBooks. Generates coding assistant configs for Crush and Kilocode so your entire AI toolchain runs locally.

## Project Structure

```text
charm-local-llm/
├── src/
│   ├── main.rs                  Entry point
│   ├── lib.rs                   Library crate (shared types / re-exports)
│   ├── cli.rs                   clap CLI definitions
│   ├── commands.rs              start/stop/status/crush/kilo/etc
│   ├── config.rs                Config struct + platform defaults
│   ├── crush.rs                 Crush config (~/.config/crush/crush.json)
│   ├── kilo_integration.rs      Kilo config patching + AGENTS.md
│   ├── modelfile.rs             Ollama modelfile parser
│   ├── ollama.rs                Ollama HTTP API client
│   └── platform.rs              Platform detection + env loading
├── platform/
│   ├── cachyos-i9-32gb-nvidia-4090/
│   ├── macos-m4-24gb/
│   ├── macos-m4-32gb/
│   ├── macos-m5-24gb/
│   └── macos-m5-32gb/           Platform env overrides + modfiles
├── tests/integration_test.rs
├── docker-compose.yml           Qdrant vector DB
├── AGENTS.md / CRUSH.md         Auto-generated project context
└── Makefile
```

## Key Commands

- `kcharm start` — Start Ollama + models + Qdrant + generate Crush/Kilo configs
- `kcharm stop` — Stop everything
- `kcharm status` — Show environment status

## Platform Detection

Auto-detected at runtime via `sysctl` (macOS) or `/etc/os-release` (Linux), or override with `--platform`:

| Platform | Directory | Memory/GPU | Primary Model | Quick Model |
|----------|-----------|------------|---------------|-------------|
| CachyOS RTX 4090 | `cachyos-i9-32gb-nvidia-4090` | 24GB VRAM | `gemma4:26b-devops` | `devstral-small-2-gpu` |
| macOS M4 24GB | `macos-m4-24gb` | 24GB unified | `qwen2.5-coder:14b-devops` | `qwen2.5-coder:7b-quick` |
| macOS M4 32GB | `macos-m4-32gb` | 32GB unified | `qwen3.6:27b-instruct-q4_K_M-devops` | `qwen2.5-coder:7b-quick` |
| macOS M5 24GB | `macos-m5-24gb` | 24GB unified | `qwen2.5-coder:14b-devops` | `qwen2.5-coder:7b-quick` |
| macOS M5 32GB | `macos-m5-32gb` | 32GB unified | `qwen3.6:27b-instruct-q4_K_M-devops` | `qwen2.5-coder:14b-quick` |

Override example: `kcharm start --platform macos-m5-32gb`

## Local LLM Setup

- **Current platform**: `cachyos` (`cachyos-i9-32gb-nvidia-4090`)
- **Memory/GPU**: NVIDIA RTX 4090 (24GB VRAM)
- **Primary coding model**: `gemma4:26b-devops`
- **Quick model**: `devstral-small-2-gpu`
- **Embeddings**: `nomic-embed-text` (768 dims)
- **Ollama**: <http://localhost:11434>
- **Qdrant**: <http://localhost:6333>
- **Specialization**: Terraform, Ansible, YAML, JSON, TypeScript/JS/Node, Go, Python, Rust

## Crush Integration

`kcharm start` generates `~/.config/crush/crush.json`:

- **Provider**: `ollama` at <http://localhost:11434/v1/> with `discover_models: true`
- **large + medium** → `gemma4:26b-devops` (8192 max tokens)
- **small** → `devstral-small-2-gpu` (4096 max tokens)
- **Context paths**: CRUSH.md, AGENTS.md, .clinerules

Also generates `CRUSH.md` in the project root as model context for Crush.

## Kilocode Integration

`kcharm start` (and `kcharm kilo init`) writes `AGENTS.md` in the project root as context for Kilocode and patches `~/.config/kilo/kilo.json`:

- Registers an `Ollama Local (FREE)` provider pointing at the local Ollama endpoint (`http://localhost:11434/v1/`) with known model aliases (including the platform devops/quick models).
- Removes any unsupported `indexing` block.

Kilocode then runs chat/inference directly against local Ollama — no external gateway, so data stays on-machine.

## Development

```bash
make build     # compile
make test      # run tests
make lint      # clippy + fmt + checkmake
make ci        # full CI pipeline
make start     # start Ollama environment
make setup     # install dependencies
```

## Code Style

- Follow Rust conventions
- Use `anyhow` for error handling
- No unnecessary comments
- Keep functions small and focused

## Integration Points

- Crush config: `~/.config/crush/crush.json` (auto-generated)
- AGENTS.md: project root context for Kilocode (auto-generated)
- CRUSH.md: project root context for Crush (auto-generated)
