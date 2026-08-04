# Troubleshooting

## Models not appearing after `make sod` / `kcharm start`

### Symptom
`kcharm start` logs "Model 'X' created successfully" and "Pulling model: Y",
but `ollama list` shows only the old pre-existing models — the new models are invisible.

### Root cause
The Ollama HTTP API endpoints (`/api/create`, `/api/pull`) return an HTTP 200
status **immediately** but stream progress lines while the server downloads and
writes the model to disk. The previous `OllamaClient` code returned as soon as
the 200 arrived **without draining the response body**, so the server was still
processing when `kcharm` moved on. The model was created on the server's
`/home/ollama/models` store, but `kcharm` reported success before the write
finished — and a subsequent `ollama list` (or even a server restart) revealed
the model was never actually persisted.

A secondary factor: `OLLAMA_MODELS` must be set to `/home/ollama/models` (the
path the systemd service uses) so the CLI and the server share the same store.
`kcharm start` now exports `OLLAMA_MODELS` into its own process environment
early, before spawning any `ollama` command.

### Fix (code-level, already applied)
1. `pull_model` and `create_model` now drain the streaming response body via
   `body.chunk().await` in a loop, ensuring the server finishes before success
   is reported.
2. `start()` exports `OLLAMA_MODELS` from `config.ollama_models_path` into
   `std::env` before any model operations.

### Environment issue: models dir ownership
On Linux the systemd service runs as the `ollama` user, so the models directory
(`/home/ollama/models`) must be owned by `ollama:ollama`. If ownership is reset
to `root:root`, the service dies with "permission denied" on its blobs dir.

One-time fix with sudo:

```bash
sudo chown -R ollama:ollama /home/ollama/models
sudo systemctl daemon-reload
sudo systemctl restart ollama
# verify:
sleep 3; ollama list
```

Then `make sod` will detect Ollama is already up and just patch the Kilo config.

### Sudoers safety
`kcharm` is hardcoded to **never** write `/etc/sudoers` or any sudoers drop-in
except the single validated `/etc/sudoers.d/ollama` (syntax-checked with
`visudo -cf` before installation). This is enforced at runtime by
`forbid_sudoers_write`, which panics on any attempt to write elsewhere under
`/etc/sudoers.d/`.
