# =============================================================================
# charm-local-llm — Rust CLI for Ollama local LLM DevOps
#   (CachyOS RTX 4090 & Apple Silicon MacBooks)
# =============================================================================
# Targets are grouped logically. The real CLI subcommands (start/stop/status/
# crush/kilo/models/qdrant/...) are provided by `kcharm` itself — invoke them
# with `make run ARGS="<subcommand> ..."` or call `kcharm` directly.
#
# Quick start:
#   make setup    # toolchain + service checks, build & install kcharm
#   make build    # compile (debug)
#   make test     # run all tests
#   make lint     # clippy + fmt check + checkmake
#   make fix      # auto-fix clippy + format
#   make ci       # full CI pipeline
# =============================================================================

# ─── Configuration ────────────────────────────────────────────────────────────
CARGO    := cargo
BIN_NAME := kcharm
PROFILE  := dev

# ─── Help / Metadata ──────────────────────────────────────────────────────────
.PHONY: help version info

help: ## Show this help message
	@echo "$(BIN_NAME) — Rust CLI for Ollama local LLM DevOps"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-15s %s\n", $$1, $$2}'
	@echo ""
	@echo "Run the CLI directly: make run ARGS=\"start\"   (or: kcharm start)"

version: ## Print version from Cargo.toml
	@$(CARGO) metadata --format-version 1 --no-deps --quiet | \
		grep -o '"version":"[^"]*"' | head -1 | cut -d'"' -f4

info: ## Print project + toolchain info
	@echo "Project: $(BIN_NAME)"
	@echo "Rust:    $$(rustc --version)"
	@echo "Cargo:   $$(cargo --version)"
	@echo "Profile: $(PROFILE)"

# ─── Setup ────────────────────────────────────────────────────────────────────
# Installs the Rust toolchain components, verifies (does not install) the
# runtime services Ollama + Docker, then builds and installs kcharm.
.PHONY: setup install setup-checks

setup: ## Install toolchain, verify services, build & install kcharm
	@command -v $(CARGO) >/dev/null 2>&1 || { echo "[error] cargo not found — install from https://rustup.rs"; exit 1; }
	@rustup component add rustfmt clippy 2>/dev/null || true
	@$(MAKE) setup-checks
	@$(MAKE) install
	@echo "[setup] Done. Run 'kcharm start' or 'make run ARGS=start'."

# Internal: verify service prerequisites (warns, never installs them).
setup-checks:
	@command -v checkmake >/dev/null 2>&1 && echo "[setup] checkmake found" || (echo "[setup] Installing checkmake..." && go install github.com/mrtazz/checkmake/cmd/checkmake@latest 2>/dev/null || echo "[warn] checkmake skipped — https://github.com/mrtazz/checkmake/releases")
	@command -v ollama >/dev/null 2>&1 && echo "[setup] Ollama found: $$(ollama --version)" || echo "[warn] Ollama not found — install from https://ollama.com (required by 'kcharm start')"
	@command -v docker >/dev/null 2>&1 && echo "[setup] Docker found: $$(docker --version)" || echo "[warn] Docker not found — required for Qdrant"

install: build ## Build and install kcharm to ~/.local/bin
	@mkdir -p ~/.local/bin
	@cp target/debug/$(BIN_NAME) ~/.local/bin/$(BIN_NAME)
	@echo "[install] $(BIN_NAME) -> ~/.local/bin/$(BIN_NAME)"
	@echo "PATH: bash/zsh export PATH=\"\$$HOME/.local/bin:\$$PATH\" | fish set -U fish_user_paths \$$HOME/.local/bin \$$fish_user_paths | pwsh [Environment]::SetEnvironmentVariable('PATH', \$$env:PATH + ';\$$HOME\.local\bin', 'User')"

# ─── Build ────────────────────────────────────────────────────────────────────
.PHONY: build build-release build-check

build: ## Compile the project (debug profile)
	$(CARGO) build --profile $(PROFILE)

build-release: ## Compile the project (release profile)
	$(CARGO) build --release

build-check: ## Type-check without emitting binaries
	$(CARGO) check --profile $(PROFILE)

# ─── Test ─────────────────────────────────────────────────────────────────────
.PHONY: test test-unit test-integration

test: ## Run all tests (unit + integration)
	$(CARGO) test --profile $(PROFILE)

test-unit: ## Run unit tests only
	$(CARGO) test --lib --profile $(PROFILE)

test-integration: ## Run integration tests only
	$(CARGO) test --test '*' --profile $(PROFILE)

# ─── Lint ─────────────────────────────────────────────────────────────────────
.PHONY: lint lint-clippy lint-fmt lint-checkmake

lint: lint-clippy lint-fmt lint-checkmake ## Run all linters (clippy + fmt + makefile)

lint-clippy: ## Run clippy, denying warnings
	$(CARGO) clippy --all-targets --all-features --profile $(PROFILE) -- -D warnings

lint-fmt: ## Check code formatting (read-only)
	$(CARGO) fmt -- --check

lint-checkmake: ## Lint this Makefile with checkmake
	@command -v checkmake >/dev/null 2>&1 && checkmake ./Makefile || echo "[warn] checkmake not installed — skipping Makefile lint"

# ─── Format / Auto-Fix ───────────────────────────────────────────────────────
.PHONY: fmt fmt-check fix fix-clippy fix-fmt

fmt: ## Auto-format code with rustfmt
	$(CARGO) fmt

fmt-check: ## Check formatting without modifying files
	$(CARGO) fmt -- --check

fix: fix-clippy fix-fmt ## Apply all automated fixes (clippy + format)

fix-clippy: ## Auto-fix clippy warnings
	$(CARGO) clippy --all-targets --all-features --profile $(PROFILE) --fix --allow-dirty

fix-fmt: ## Auto-format code
	$(CARGO) fmt

# ─── Dependencies ─────────────────────────────────────────────────────────────
.PHONY: deps deps-update deps-check

deps: ## Write dependency tree to deps.txt
	$(CARGO) tree > deps.txt
	@echo "[deps] Dependency tree written to deps.txt"

deps-update: ## Update deps to latest compatible versions
	$(CARGO) update
	@command -v cargo-outdated >/dev/null 2>&1 && { $(CARGO) outdated > deps-outdated.txt; echo "[deps-update] Compatible deps updated; any major updates written to deps-outdated.txt (edit Cargo.toml to apply)"; } || \
		echo "[deps-update] Compatible deps updated. Run 'make deps-check' to see major updates."

deps-check: ## Write outdated-dependency report to deps-outdated.txt
	@command -v cargo-outdated >/dev/null 2>&1 && { $(CARGO) outdated > deps-outdated.txt; echo "[deps-check] Report written to deps-outdated.txt"; } || \
		echo "[warn] cargo-outdated not installed — run 'cargo install cargo-outdated' (one-time, compiles from source) to enable"

# ─── Run ──────────────────────────────────────────────────────────────────────
.PHONY: run

run: ## Run the CLI: make run ARGS="start --platform-override cachyos"
	$(CARGO) run --profile $(PROFILE) -- $(ARGS)

# ─── Cleanup ──────────────────────────────────────────────────────────────────
.PHONY: clean clean-all

clean: ## Remove build artifacts (cargo clean)
	$(CARGO) clean

clean-all: clean ## Also remove the target directory
	rm -rf target/

# ─── CI / Aggregates ──────────────────────────────────────────────────────────
.PHONY: ci all pre-commit

all: lint test ## Run lint and tests

ci: ## Full CI pipeline (quiet lint + tests)
	$(CARGO) clippy --all-targets --all-features --profile $(PROFILE) -q -- -D warnings
	$(CARGO) fmt -- --check -q
	$(CARGO) test --profile $(PROFILE) --quiet

pre-commit: fmt-check lint-clippy test-unit ## Run pre-commit checks locally

# ─── Default ──────────────────────────────────────────────────────────────────
.DEFAULT_GOAL := help
