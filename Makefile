.PHONY: help all clean test build release lint fmt check-fmt markdownlint nixie


TARGET ?= libactix_v2a.rlib

PREPEND_PATH := $(HOME)/.cargo/bin:$(HOME)/.bun/bin:$(HOME)/.local/bin

CARGO ?= $(shell PATH=$(PREPEND_PATH):$(PATH) command -v cargo 2>/dev/null || printf '%s/.cargo/bin/cargo' "$$HOME")
BUILD_JOBS ?=
RUST_FLAGS ?=
RUST_FLAGS := -D warnings $(RUST_FLAGS)
RUSTDOC_FLAGS ?=
RUSTDOC_FLAGS := -D warnings $(RUSTDOC_FLAGS)
CARGO_FLAGS ?= --all-targets --all-features
CLIPPY_FLAGS ?= $(CARGO_FLAGS) -- $(RUST_FLAGS)
TEST_FLAGS ?= $(CARGO_FLAGS)
TEST_CMD := $(if $(shell PATH=$(PREPEND_PATH):$(PATH) $(CARGO) nextest --version 2>/dev/null),nextest run,test)
MDLINT ?= $(shell PATH=$(PREPEND_PATH):$(PATH) command -v markdownlint-cli2 2>/dev/null || printf '%s/.bun/bin/markdownlint-cli2' "$$HOME")
NIXIE ?= $(shell PATH=$(PREPEND_PATH):$(PATH) command -v nixie 2>/dev/null || printf '%s/.bun/bin/nixie' "$$HOME")

build: target/debug/$(TARGET) ## Build debug binary
release: target/release/$(TARGET) ## Build release binary

all: check-fmt lint test ## Perform a comprehensive check of code

clean: ## Remove build artifacts
	PATH="$(PREPEND_PATH):$(PATH)" $(CARGO) clean

test: ## Run tests with warnings treated as errors
	PATH="$(PREPEND_PATH):$(PATH)" RUSTFLAGS="$(RUST_FLAGS)" $(CARGO) $(TEST_CMD) $(TEST_FLAGS) $(BUILD_JOBS)
ifneq ($(TEST_CMD),test)
	PATH="$(PREPEND_PATH):$(PATH)" RUSTFLAGS="$(RUST_FLAGS)" $(CARGO) test --doc --workspace --all-features
endif

target/%/$(TARGET): ## Build binary in debug or release mode
	PATH="$(PREPEND_PATH):$(PATH)" $(CARGO) build $(BUILD_JOBS) $(if $(findstring release,$(@)),--release)

lint: ## Run Clippy with warnings denied
	PATH="$(PREPEND_PATH):$(PATH)" RUSTDOCFLAGS="$(RUSTDOC_FLAGS)" $(CARGO) doc --no-deps
	PATH="$(PREPEND_PATH):$(PATH)" $(CARGO) clippy $(CLIPPY_FLAGS)
	@if PATH="$(PREPEND_PATH):$(PATH)" command -v whitaker >/dev/null 2>&1; then \
		PATH="$(PREPEND_PATH):$(PATH)" RUSTFLAGS="$(RUST_FLAGS)" whitaker --all -- $(CARGO_FLAGS); \
	else \
		echo "whitaker not found on PATH; skipping whitaker lint. Install whitaker to run this check."; \
	fi

typecheck: ## Type-check without building
	PATH="$(PREPEND_PATH):$(PATH)" RUSTFLAGS="$(RUST_FLAGS)" $(CARGO) check $(CARGO_FLAGS)

fmt: ## Format Rust and Markdown sources
	PATH="$(PREPEND_PATH):$(PATH)" $(CARGO) +nightly fmt --all
	mdformat-all

check-fmt: ## Verify formatting
	PATH="$(PREPEND_PATH):$(PATH)" $(CARGO) fmt --all -- --check

markdownlint: ## Lint Markdown files
	PATH="$(PREPEND_PATH):$(PATH)" $(MDLINT) '**/*.md'

nixie: ## Validate Mermaid diagrams
	PATH="$(PREPEND_PATH):$(PATH)" $(NIXIE) --no-sandbox

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?##' $(MAKEFILE_LIST) | \
	awk 'BEGIN {FS=":"; printf "Available targets:\n"} {printf "  %-20s %s\n", $$1, $$2}'
