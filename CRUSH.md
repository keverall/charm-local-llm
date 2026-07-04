# Project Context

## Environment
- **Platform**: cachyos
- **Ollama**: http://localhost:11434 (local, free, no rate limits)
- **Primary LLM**: `gemma4:26b-devops` (RTX 4090, 24GB VRAM)
- **Quick model**: `devstral-small-2-gpu`
- **Vector DB**: Qdrant at http://localhost:6333
- **Embeddings**: `nomic-embed-text` (768 dimensions)

## Models Available (GPU-optimized)
| Model | Purpose | VRAM |
|-------|---------|------|
| `gemma4:26b-devops` | Primary coding, complex reasoning | ~18GB |
| `devstral-small-2-gpu` | Fast responses, simple tasks | ~15GB |
| `nomic-embed-text` | Embeddings for RAG/search | ~300MB |
| `gemma4:26b` | Alternative coding model | ~17GB |

## Guidelines
- Prefer local Ollama models for all development tasks
- Use `gemma4:26b-devops` for complex code generation, architecture, and debugging
- Use `devstral-small-2-gpu` for simple edits, questions, and fast iterations
- When using local models, data never leaves this machine
- GPU optimizations are configured for maximum throughput on RTX 4090
