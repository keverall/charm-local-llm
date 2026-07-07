# =============================================================================
# charm-local-llm — Rust CLI for Ollama local LLM DevOps (CachyOS RTX 4090)
# =============================================================================
# Common tasks for local development and CI/CD.
#
# Quick start:
#   make setup    # Install deps and verify tools
#   make build    # Compile the project
#   make test     # Run tests
#   make lint     # Run clippy + format check + checkmake
#   make fix      # Auto-fix clippy warnings and format
#   make ci       # Full CI pipeline (lint + test)
# =============================================================================

# ─── Configuration ────────────────────────────────────────────────────────────
CARGO      := cargo
BIN_NAME   := kcharm
PROFILE    := dev

# ─── Metadata ─────────────────────────────────────────────────────────────────
.PHONY: help version info

help: ## Show this help message
	@echo "$(BIN_NAME) — Rust CLI for Ollama local LLM DevOps"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  %-15s %s\n", $$1, $$2}'
	@echo ""

version: ## Print version from Cargo.toml
	@$(CARGO) metadata --format-version 1 --no-deps --quiet | \
		grep -o '"version":"[^"]*"' | head -1 | cut -d'"' -f4

info: ## Print project info
	@echo "Project:   $(BIN_NAME)"
	@echo "Rust:     $$(rustc --version)"
	@echo "Cargo:    $$(cargo --version)"
	@echo "Profile:  $(PROFILE)"

# ─── Dependency Setup ──────────────────────────────────────────────────────────
.PHONY: setup check-deps setup-tools setup-checkmake setup-ollama setup-docker setup-install setup-fish setup-powershell install

setup: check-deps setup-tools setup-checkmake setup-ollama setup-docker setup-install ## Install deps, build, and install kcharm
	@echo "[setup] Done. Run 'kcharm start' or 'make run-start'"

setup-tools: ## Ensure Rust toolchain, rustfmt, and clippy
	@rustc --version >/dev/null 2>&1 || (echo "[error] rustc not found. Install from https://rustup.rs" && exit 1)
	@rustup component add rustfmt clippy 2>/dev/null || true
	@echo "[setup] rustfmt + clippy ready"

setup-checkmake: ## Install checkmake if missing
	@command -v checkmake >/dev/null 2>&1 && echo "[setup] checkmake found" || \
		(echo "[setup] Installing checkmake..." && \
		 go install github.com/mrtazz/checkmake/cmd/checkmake@latest 2>/dev/null || \
		 echo "[warn] checkmake not installed — https://github.com/mrtazz/checkmake/releases")

setup-ollama: ## Check Ollama installation
	@command -v ollama >/dev/null 2>&1 && echo "[setup] Ollama found: $$(ollama --version)" || \
		echo "[warn] Ollama not found — install from https://ollama.com"

setup-docker: ## Check Docker and docker-compose
	@command -v docker >/dev/null 2>&1 && echo "[setup] Docker found: $$(docker --version)" || \
		echo "[warn] Docker not found — Qdrant requires Docker"
	@command -v docker-compose >/dev/null 2>&1 && echo "[setup] docker-compose found" || \
		echo "[warn] docker-compose not found — Qdrant requires docker-compose"

setup-install: ## Build and install kcharm to ~/.local/bin
	@$(MAKE) build
	@mkdir -p ~/.local/bin
	@cp target/debug/$(BIN_NAME) ~/.local/bin/$(BIN_NAME)
	@echo "[setup] Installed $(BIN_NAME) to ~/.local/bin/"
	@echo "Add to PATH: Fish: set -U fish_user_paths ~/.local/bin \$$fish_user_paths"

setup-fish: ## Add kcharm to fish PATH
	@mkdir -p ~/.local/bin
	@$(MAKE) build
	@cp target/debug/$(BIN_NAME) ~/.local/bin/$(BIN_NAME)
	@fish -c "set -U fish_user_paths ~/.local/bin \$$fish_user_paths" 2>/dev/null || \
		echo "[setup] kcharm installed. Restart fish or run: set -U fish_user_paths ~/.local/bin \$$fish_user_paths"

setup-powershell: ## Add kcharm to PowerShell PATH
	@mkdir -p ~/.local/bin
	@$(MAKE) build
	@cp target/debug/$(BIN_NAME) ~/.local/bin/$(BIN_NAME)
	@echo "[setup] kcharm installed to ~/.local/bin/"
	@echo "PowerShell: [Environment]::SetEnvironmentVariable('PATH', \$$env:PATH + ';\$$HOME\.local\bin', 'User')"

check-deps:
	@if ! command -v $(CARGO) >/dev/null 2>&1; then \
		echo "[error] Rust/cargo not found. Install from https://rustup.rs"; \
		exit 1; \
	fi

# ─── Build ────────────────────────────────────────────────────────────────────
.PHONY: build build-release build-check

build: ## Compile the project (debug profile)
	$(CARGO) build --profile $(PROFILE)

build-release: ## Compile the project (release profile)
	$(CARGO) build --release

build-check: ## Type-check without building
	$(CARGO) check --profile $(PROFILE)

# ─── Testing ──────────────────────────────────────────────────────────────────
.PHONY: test test-unit test-integration test-ci

test: ## Run all tests (unit + integration)
	$(CARGO) test --profile $(PROFILE)

test-unit: ## Run unit tests only
	$(CARGO) test --lib --profile $(PROFILE)

test-integration: ## Run integration tests only
	$(CARGO) test --test '*' --profile $(PROFILE)

test-ci: test ## Run tests in CI mode
	$(CARGO) test --profile $(PROFILE) --quiet

# ─── Linting ──────────────────────────────────────────────────────────────────
.PHONY: lint lint-clippy lint-fmt lint-ci lint-checkmake

lint: lint-clippy lint-fmt lint-checkmake ## Run all linters (clippy + format check + makefile)

lint-clippy: ## Run clippy with strict warnings
	$(CARGO) clippy --all-targets --all-features --profile $(PROFILE) -- -D warnings

lint-fmt: ## Check code formatting (read-only)
	$(CARGO) fmt -- --check

lint-ci: ## CI-only lint (no output unless it fails)
	$(CARGO) clippy --all-targets --all-features --profile $(PROFILE) -q -- -D warnings
	$(CARGO) fmt -- --check -q

lint-checkmake: ## Lint Makefile with checkmake
	@command -v checkmake >/dev/null 2>&1 && checkmake ./Makefile || echo "[warn] checkmake not installed — skipping Makefile lint"

# ─── Formatting ───────────────────────────────────────────────────────────────
.PHONY: fmt fmt-check

fmt: ## Auto-format code with rustfmt
	$(CARGO) fmt

fmt-check: ## Check formatting without modifying files
	$(CARGO) fmt -- --check

# ─── Auto-Fix ─────────────────────────────────────────────────────────────────
.PHONY: fix fix-clippy fix-fmt

fix: fix-clippy fix-fmt ## Apply all automated fixes (clippy + format)

fix-clippy: ## Auto-fix clippy warnings
	$(CARGO) clippy --all-targets --all-features --profile $(PROFILE) --fix --allow-dirty

fix-fmt: ## Auto-format code
	$(CARGO) fmt

# ─── Dependencies ─────────────────────────────────────────────────────────────
.PHONY: deps deps-update deps-check

deps: ## Show dependency tree
	$(CARGO) tree --profile $(PROFILE)

deps-update: ## Update dependencies to latest compatible versions
	$(CARGO) update

deps-check: ## Check for outdated dependencies
	$(CARGO) outdated || echo "[warn] cargo-outdated not installed — cargo install cargo-outdated"

# ─── Run ──────────────────────────────────────────────────────────────────────
.PHONY: run run-start run-stop run-status run-service run-models run-qdrant

run: ## Run the CLI: make run ARGS="start"
	$(CARGO) run --profile $(PROFILE) -- $(ARGS)

run-start: ## Start Ollama environment (generates Crush + Kilo config)
	$(CARGO) run --profile $(PROFILE) -- start

run-stop: ## Stop Ollama environment
	$(CARGO) run --profile $(PROFILE) -- stop

run-status: ## Show environment status
	$(CARGO) run --profile $(PROFILE) -- status

run-service: ## Manage Ollama systemd service
	$(CARGO) run --profile $(PROFILE) -- service $(ARGS)

run-models: ## Manage models
	$(CARGO) run --profile $(PROFILE) -- models $(ARGS)

run-qdrant: ## Manage Qdrant container
	$(CARGO) run --profile $(PROFILE) -- qdrant $(ARGS)

install: setup-install ## Build and install kcharm to ~/.local/bin

# ─── Crusher (Crush) ─────────────────────────────────────────────────────────
.PHONY: crush-init crush-status crush-context

crush-init: ## Generate Crush config for Ollama on ~/.config/crush/crush.json
	$(CARGO) run --profile $(PROFILE) -- crush init

crush-status: ## Show Crush config status
	$(CARGO) run --profile $(PROFILE) -- crush status

crush-context: ## Generate CRUSH.md project context file
	$(CARGO) run --profile $(PROFILE) -- crush context

# ─── Kilocode ─────────────────────────────────────────────────────────────────
.PHONY: kilo-init kilo-status kilo-context

kilo-init: ## Patch Kilocode indexing config for Ollama + Qdrant
	$(CARGO) run --profile $(PROFILE) -- kilo init

kilo-status: ## Show Kilocode indexing config status
	$(CARGO) run --profile $(PROFILE) -- kilo status

kilo-context: ## Generate AGENTS.md project context file
	$(CARGO) run --profile $(PROFILE) -- kilo context

# ─── Cleanup ──────────────────────────────────────────────────────────────────
.PHONY: clean clean-all clean-target

clean: ## Remove build artifacts
	$(CARGO) clean

clean-all: clean ## Remove build artifacts and target directory
	rm -rf target/

clean-target: ## Remove target directory only
	rm -rf target/

# ─── CI / Aggregates ──────────────────────────────────────────────────────────
.PHONY: ci all pre-commit

all: lint test ## Run lint and tests

ci: lint-ci test-ci ## Run full CI pipeline

pre-commit: fmt-check lint-clippy test-unit ## Run pre-commit checks locally

# ─── Default ──────────────────────────────────────────────────────────────────
.DEFAULT_GOAL := help
