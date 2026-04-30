.PHONY: help all clean test build release lint fmt check-fmt markdownlint nixie


TARGET ?= libactix_v2a.rlib

CARGO ?= cargo
CARGO_BIN ?= $(HOME)/.cargo/bin
CARGO_ENV := PATH="$(CARGO_BIN):$$PATH"
BUILD_JOBS ?=
RUST_FLAGS ?=
RUST_FLAGS := -D warnings $(RUST_FLAGS)
RUSTDOC_FLAGS ?=
RUSTDOC_FLAGS := -D warnings $(RUSTDOC_FLAGS)
CARGO_FLAGS ?= --all-targets --all-features
CLIPPY_FLAGS ?= $(CARGO_FLAGS) -- $(RUST_FLAGS)
TEST_FLAGS ?= $(CARGO_FLAGS)
TEST_CMD := $(if $(shell $(CARGO_ENV) $(CARGO) nextest --version 2>/dev/null),nextest run,test)
MDLINT ?= markdownlint-cli2
BUN_BIN ?= $(HOME)/.bun/bin
NIXIE ?= nixie

build: target/debug/$(TARGET) ## Build debug binary
release: target/release/$(TARGET) ## Build release binary

all: check-fmt lint test ## Perform a comprehensive check of code

clean: ## Remove build artifacts
	$(CARGO_ENV) $(CARGO) clean

test: ## Run tests with warnings treated as errors
	$(CARGO_ENV) RUSTFLAGS="$(RUST_FLAGS)" $(CARGO) $(TEST_CMD) $(TEST_FLAGS) $(BUILD_JOBS)
ifneq ($(TEST_CMD),test)
	$(CARGO_ENV) RUSTFLAGS="$(RUST_FLAGS)" $(CARGO) test --doc --workspace --all-features
endif

target/%/$(TARGET): ## Build binary in debug or release mode
	$(CARGO_ENV) $(CARGO) build $(BUILD_JOBS) $(if $(findstring release,$(@)),--release)

lint: ## Run Clippy with warnings denied
	$(CARGO_ENV) RUSTDOCFLAGS="$(RUSTDOC_FLAGS)" $(CARGO) doc --no-deps
	$(CARGO_ENV) $(CARGO) clippy $(CLIPPY_FLAGS)
	@if command -v whitaker >/dev/null 2>&1; then \
		$(CARGO_ENV) RUSTFLAGS="$(RUST_FLAGS)" whitaker --all -- $(CARGO_FLAGS); \
	else \
		echo "whitaker not found on PATH; skipping whitaker lint. Install whitaker to run this check."; \
	fi

typecheck: ## Type-check without building
	$(CARGO_ENV) RUSTFLAGS="$(RUST_FLAGS)" $(CARGO) check $(CARGO_FLAGS)

fmt: ## Format Rust and Markdown sources
	$(CARGO_ENV) $(CARGO) +nightly fmt --all
	mdformat-all

check-fmt: ## Verify formatting
	$(CARGO_ENV) $(CARGO) fmt --all -- --check

markdownlint: ## Lint Markdown files
	PATH="$(BUN_BIN):$$PATH" $(MDLINT) '**/*.md'

nixie: ## Validate Mermaid diagrams
	$(NIXIE) --no-sandbox

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?##' $(MAKEFILE_LIST) | \
	awk 'BEGIN {FS=":"; printf "Available targets:\n"} {printf "  %-20s %s\n", $$1, $$2}'
