# Repository layout

This document maps the top-level tree and explains where major kinds of files
belong in `actix-v2a`. It is the canonical location for repository layout and
path-responsibility guidance.

## Overview

The tree below is intentionally abbreviated. It shows the paths that matter to
day-to-day contributors and omits generated or implementation-specific detail.

```plaintext
.
├── AGENTS.md
├── Cargo.toml
├── Cargo.lock
├── Makefile
├── Netsukefile
├── README.md
├── clippy.toml
├── codecov.yml
├── docs/
├── rust-toolchain.toml
├── src/
└── tests/
```

## Top-level files

Table: Top-level repository files and their responsibilities.

| Path | Responsibility |
| --- | --- |
| `AGENTS.md` | Repository instructions for agents and maintainers. |
| `Cargo.toml` | Workspace manifest and crate metadata. |
| `Cargo.lock` | Locked dependency versions for reproducible builds. |
| `Makefile` | Stable task entry points for build, lint, and formatting workflows. |
| `Netsukefile` | Build-driver configuration consumed by `netsuke`. |
| `README.md` | Public overview and first-stop entry point for new readers. |
| `clippy.toml` | Clippy configuration and lint policy. |
| `codecov.yml` | Coverage configuration. |
| `rust-toolchain.toml` | Pinned Rust toolchain selection. |

## Source code

The `src/` tree holds the library implementation. The code is organised by
feature rather than by technical layer:

- `src/lib.rs` is the crate root and re-export surface.
- `src/error.rs` holds shared error types.
- `src/http/` contains HTTP-facing adapters and helpers.
- `src/idempotency/` contains idempotency-key helpers and related contract
  code.
- `src/openapi/` contains OpenAPI generation or description logic.
- `src/pagination/` contains pagination helpers and wire-contract logic.
- `src/sse/` contains Server-Sent Events helpers and adapters.

Keep new shared behaviour close to the owning feature area unless there is a
clear reason to introduce a new cross-cutting module.

## Tests

The `tests/` tree holds integration, BDD-style, and snapshot coverage:

- `tests/features/` contains reusable BDD feature files.
- `tests/support/` contains shared test harness helpers.
- `tests/snapshots/` contains checked-in snapshot expectations.
- `tests/*_bdd.rs` and `tests/*_tests.rs` cover externally observable behaviour.

Prefer adding tests beside the feature they exercise. Shared fixtures belong
in `tests/support/` rather than in ad hoc helpers spread across suites.

## Documentation

The `docs/` tree holds the long-lived project knowledge base:

- `docs/contents.md` is the index for the documentation set.
- `docs/documentation-style-guide.md` defines writing and formatting rules.
- `docs/developers-guide.md` documents maintainer-facing workflows and
  conventions.
- `docs/users-guide.md` describes user-facing behaviour and workflows.
- `docs/repository-layout.md` explains where files and directories belong.
- `docs/adr-*.md` files capture accepted architectural decisions.
- `docs/*-design.md` files explain the shape and rationale of major systems.
- `docs/roadmap.md` records the broader delivery backlog.
- `docs/execplans/` contains living execution plans and retrospectives.

Keep design intent, decision records, and procedural guidance in the most
specific document that can own it. Avoid duplicating repository layout
guidance outside `docs/repository-layout.md`.

## Tooling and automation

The root configuration files control the local toolchain and repository
policies:

- `.markdownlint-cli2.jsonc` configures Markdown linting.
- `.rustfmt.toml` configures Rust formatting.
- `.github/` contains CI workflows and other GitHub automation.
- `.agents/` contains agent-tooling state managed by the connected
  automation.

Treat these paths as infrastructure rather than product code. Update them when
the tooling or automation contract changes, not as part of ordinary feature
work.
