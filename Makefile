.PHONY: help all clean test build release lint fmt check-fmt markdownlint nixie typecheck spelling test-workflow-contracts


PREPEND_PATH := $(HOME)/.cargo/bin:$(HOME)/.bun/bin:$(HOME)/.local/bin
NETSUKE ?= $(shell PATH=$(PREPEND_PATH):$(PATH) command -v netsuke 2>/dev/null || printf '%s/.cargo/bin/netsuke' "$$HOME")
CARGO ?= cargo
UV ?= uv
UV_ENV = UV_CACHE_DIR=.uv-cache UV_TOOL_DIR=.uv-tools

# The CV-005 CodeScene contracts live in shared-actions and run from a full
# commit, so a fix is a pin bump. `.github/cv005.toml` holds this repository's
# only parameters.
CV005_CONTRACTS_REF ?= 88977798a5c3bae1549afb99642529488c665276
CV005_CONTRACTS = $(UV_ENV) $(UV) tool run --python 3.13 \
	--from 'git+https://github.com/leynos/shared-actions@$(CV005_CONTRACTS_REF)\#subdirectory=packages/cv005-contracts' \
	cv005-contracts

TYPOS_CONFIG_BUILDER_VERSION ?= v0.1.3
TYPOS_CONFIG_BUILDER = $(UV_ENV) $(UV) tool run --python 3.14 --from \
	"git+https://github.com/leynos/typos-config-builder.git@$(TYPOS_CONFIG_BUILDER_VERSION)" \
	typos-config-builder

MDLINT ?= $(shell command -v markdownlint-cli2 2>/dev/null || printf '%s' "$$HOME/.bun/bin/markdownlint-cli2")
# `make fmt` and `make check-fmt` call mdtablefix directly. `--git` selects the
# Markdown files Git tracks and `--include-untracked` adds the untracked files
# Git does not ignore, so a new document is formatted before it is staged.
# Both modes need mdtablefix 0.6.0 or later; CI pins the version at the
# install-mdtablefix step.
MDTABLEFIX ?= mdtablefix
MDTABLEFIX_SELECT = --git --include-untracked
MDTABLEFIX_RULES = --wrap --renumber --breaks --ellipsis --fences

test-workflow-contracts: ## Check the CV-005 CodeScene workflow contracts
	$(CV005_CONTRACTS) check --repository .

build: ## Build debug artefacts
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build build

release: ## Build release artefacts
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build release

all: spelling test-workflow-contracts ## Perform a comprehensive check of code
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build all

clean: ## Remove build artefacts
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build clean

test: ## Run tests with warnings treated as errors
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build test

lint: ## Run Clippy and the Whitaker Dylint suite with warnings denied
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build lint

typecheck: ## Type-check without building
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build typecheck

fmt: ## Format Rust and Markdown sources
	$(CARGO) fmt --all
	$(MDTABLEFIX) --in-place $(MDTABLEFIX_SELECT) $(MDTABLEFIX_RULES)
	$(MDLINT) --fix "**/*.md"

check-fmt: ## Verify formatting
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build check-fmt
	$(MDTABLEFIX) --check $(MDTABLEFIX_SELECT) $(MDTABLEFIX_RULES)

markdownlint: spelling ## Lint Markdown files and enforce spelling
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build markdownlint

spelling: ## Enforce en-GB-oxendict spelling
	$(TYPOS_CONFIG_BUILDER) gate --repository .

nixie: ## Validate Mermaid diagrams
	PATH="$(PREPEND_PATH):$(PATH)" $(NETSUKE) build nixie

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?##' $(MAKEFILE_LIST) | \
	awk 'BEGIN {FS=":"; printf "Available targets:\n"} {printf "  %-20s %s\n", $$1, $$2}'
