# charm-local-llm

Rust CLI for Ollama local LLM DevOps — automates setup, optimization, and lifecycle management of Ollama on CachyOS with NVIDIA RTX 4090.

## Architecture
- **Rust CLI** using `clap` for subcommands
- **Ollama** for local LLM inference (RTX 4090 optimized)
- **Qdrant** for vector database (semantic search)
- **Crush** integration for agentic coding with local models

## Key Commands
- `charm start` — Start Ollama + models + Qdrant + configure Crush
- `charm stop` — Stop everything
- `charm status` — Show environment status

## Local LLM Setup
- **Primary coding model**: `gemma4:26b-devops` (RTX 4090, 24GB VRAM)
- **Quick model**: `devstral-small-2-gpu`
- **Embeddings**: `nomic-embed-text` (768 dims)
- **Ollama**: http://localhost:11434
- **Qdrant**: http://localhost:6333

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
- Crush config: `~/.config/crush/crush.json` (auto-generated on `charm start`)
- Kilo indexing: `~/.config/kilo/kilo.json` (Ollama + Qdrant)
- CRUSH.md: Project root context file (auto-generated)
