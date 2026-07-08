# Local Ollama Development Environment

## Platform

- **Detected platform**: Auto-detected at runtime or via `--platform` override
- **Memory architecture**: Platform-specific (see AGENTS.md for details)

## Primary LLM

- **Large/Medium**: `{devops}`
- **Small/Quick**: `{quick}`
- **Provider**: Ollama Local at <http://localhost:{port}/v1/>
- **Embeddings**: `nomic-embed-text` (768 dimensions)
- **Qdrant**: <http://localhost:{qdrant}>

## Specialization

This environment is optimized for DevOps and software development workflows:

- **Infrastructure as Code**: Terraform, Ansible
- **Configuration**: YAML, JSON
- **Backend/Systems**: Go, Python, Rust
- **Web/Frontend**: TypeScript, JavaScript, Node.js

## Guidelines

- Prefer local Ollama models for all development tasks
- Use the primary model (`{devops}`) for complex code generation, architecture, and debugging
- Use the quick model (`{quick}`) for simple edits, questions, and fast iterations
- When using local models, data never leaves this machine
- Platform-specific optimizations are configured for maximum throughput

## Crush Config

- Config path: `~/.config/crush/crush.json`
- Provider: `ollama` at <http://localhost:{port}/v1/> with `discover_models: true`
- **large / medium** → `{devops}` (8192 max tokens)
- **small** → `{quick}` (4096 max tokens)
- Context paths: CRUSH.md, AGENTS.md, .clinerules

## Kilocode Integration

- Config path: `~/.config/kilo/kilo.json`
- `kcharm` registers an `Ollama Local (FREE)` provider in kilo.json pointing at the local Ollama endpoint, with known model aliases (including the platform devops/quick models).
- Kilocode runs chat/inference directly against local Ollama — no external gateway, data stays on-machine.
