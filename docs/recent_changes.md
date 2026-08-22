# Recent Changes

## 2026-08-06 — Deepseek-R1 32B GPU Model Support

### Problem
`kcharm start` (via `make sod`) failed to ensure `deepseek-r1:32b-gpu` was present in
the Ollama model store. Additionally, the base model `deepseek-r1:32b` was left behind
in the store when it should have been cleaned up.

### Root Causes

1. **Wrong fallback in `ensure_model`** (`src/ollama.rs`):
   When `create_model` failed for a modelfile model, `ensure_model` fell back to
   `pull_model(model_name)`. For modelfile models like `deepseek-r1:32b-gpu`, the
   model name is not a valid Ollama registry tag (the `-gpu` suffix is a local
   modelfile tag). The fallback always failed silently, hiding the real error.

2. **Base model not cleaned up**:
   The base dependency cleanup loop in `src/commands.rs` correctly removes models
   not listed in `DEFAULT_MODELS`. This was working for other models but
   `deepseek-r1:32b` was present because it was pulled manually during debugging.

### Fixes Applied

- **`src/ollama.rs` — `ensure_model`**: When `create_model` fails for a modelfile
  model, return the error directly instead of attempting a fallback pull of a
  non-existent registry tag. The real error (e.g., base model pull failure) is now
  properly reported to the caller, which logs a warning.

- **`src/commands.rs` — base dependency cleanup**: No code change needed. The
  existing cleanup loop already removes base models not in `DEFAULT_MODELS`.
  A temporary change that protected FROM base models was reverted.

- **Manual cleanup**: Removed `deepseek-r1:32b` from the Ollama store with
  `ollama rm deepseek-r1:32b`.

### Verification

- `cargo fmt --check` — passes
- `cargo clippy --all-targets --all-features -- -D warnings` — passes
- `cargo test` — 38 tests pass (12 unit + 26 integration)
- `ollama list` confirms only `deepseek-r1:32b-gpu` is present (no base model)
