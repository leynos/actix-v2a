.PHONY: help all clean test build release lint fmt check-fmt markdownlint nixie typecheck


PREPEND_PATH := $(HOME)/.cargo/bin:$(HOME)/.bun/bin:$(HOME)/.local/bin
NETSUKE ?= $(shell PATH=$(PREPEND_PATH):$(PATH) command -v netsuke 2>/dev/null || printf '%s/.cargo/bin/netsuke' "$$HOME")

build: ## Build debug artefacts
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build build

release: ## Build release artefacts
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build release

all: ## Perform a comprehensive check of code
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build all

clean: ## Remove build artefacts
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build clean

test: ## Run tests with warnings treated as errors
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build test

lint: ## Run Clippy with warnings denied
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build lint

typecheck: ## Type-check without building
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build typecheck

fmt: ## Format Rust and Markdown sources
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build fmt

check-fmt: ## Verify formatting
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build check-fmt

markdownlint: ## Lint Markdown files
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build markdownlint

nixie: ## Validate Mermaid diagrams
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build nixie

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?##' $(MAKEFILE_LIST) | \
	awk 'BEGIN {FS=":"; printf "Available targets:\n"} {printf "  %-20s %s\n", $$1, $$2}'
