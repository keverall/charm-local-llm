# charm-local-llm

Rust CLI that automates setup, optimization, and lifecycle management of local Ollama LLMs on CachyOS RTX 4090 and Apple Silicon MacBooks. Generates coding assistant configs for Crush and Kilocode so your entire AI toolchain runs locally.

## Project Structure

```text
charm-local-llm/
├── src/
│   ├── main.rs                  Entry point
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

- `charm start` — Start Ollama + models + Qdrant + generate Crush/Kilo configs
- `charm stop` — Stop everything
- `charm status` — Show environment status

## Local LLM Setup

- **CachyOS RTX 4090 primary**: `gemma4:26b-devops` / `qwen3-coder:30b-gpu`
- **MacBook M4/M5 24GB primary**: `qwen2.5-coder:14b-devops`
- **MacBook M4 32GB primary**: `qwen3-coder:30b-devops`, quick `qwen2.5-coder:7b-quick`
- **MacBook M5 32GB primary**: `qwen3-coder:30b-devops`, quick `qwen2.5-coder:14b-quick`
- **Embeddings**: `nomic-embed-text` (768 dims)
- **Ollama**: <http://localhost:11434>
- **Qdrant**: <http://localhost:6333>
- **Specialization**: Terraform, Ansible, YAML, JSON, TypeScript/JS/Node, Go, Python, Rust

## Crush Integration

`charm start` generates `~/.config/crush/crush.json`:

- **Provider**: `ollama` at <http://localhost:11434/v1/> with `discover_models: true`
- **large + medium** → `gemma4:26b-devops` (8192 max tokens)
- **small** → `devstral-small-2-gpu` (4096 max tokens)
- **Context paths**: CRUSH.md, AGENTS.md, .clinerules

Also generates `CRUSH.md` in the project root as model context for Crush.

## Kilocode Integration

`charm start` generates `AGENTS.md` in the project root as context for Kilocode.

Kilo chat models route through the Kilo Gateway. Local Ollama is used only for chat model inference via the Gateway when local models are selected.

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
