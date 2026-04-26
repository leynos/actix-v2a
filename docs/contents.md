# Documentation contents

- [Documentation contents](contents.md): entry point for the documentation
  set, including guides, references, and execution plans.
- [Documentation style guide](documentation-style-guide.md): repository
  standards for structure, tone, formatting, and maintenance of documentation.
- [User's guide](users-guide.md): usage documentation for the `actix-v2a`
  crate's public API, including Server-Sent Events (SSE) helpers, error
  handling, and integration patterns.
- [Developer's guide](developers-guide.md): internal conventions, module
  structure, quality gates, and contribution guidance for `actix-v2a`
  maintainers.
- [Roadmap](roadmap.md): phased implementation backlog for planned shared
  `actix-v2a` capabilities and follow-on delivery work.
- [Complexity antipatterns and refactoring strategies](complexity-antipatterns-and-refactoring-strategies.md):
  reference material for identifying and reducing structural complexity in code.
- [Reliable testing in Rust via dependency injection](reliable-testing-in-rust-via-dependency-injection.md):
  guidance for designing testable Rust components without mutating global
  process state.
- [Rust doctest DRY guide](rust-doctest-dry-guide.md): reference for writing
  maintainable Rust documentation tests without duplication.
- [Rust testing with rstest fixtures](rust-testing-with-rstest-fixtures.md):
  guidance for shared Rust test fixtures, parameterized cases, and scenario
  setup.
- [Scripting standards](scripting-standards.md): reference for Python and shell
  scripting conventions used in this repository.
- [Architectural decision record (ADR) 001: shared SSE wire
  contract for Wildside and
  Corbusier](adr-001-shared-sse-wire-contract-for-wildside-and-corbusier.md):
  proposed shared Server-Sent Events (SSE) contract for cross-application
  streaming helpers in `actix-v2a`.
- `execplans/`: living execution plans for non-trivial change work.
  - [Port Wildside pagination documentation
    hardening](execplans/portwildsidepagination.md): draft plan for adapting
    Wildside commit `9d6b7655` pagination documentation and invariant tests
    into `actix-v2a`.
  - [Import components from Wildside](execplans/import-components-from-wildside.md):
    draft plan for extracting shared HTTP and API primitives from
    `../wildside/backend` into this crate.
  - [Implement SSE identifier and replay cursor
    helpers](execplans/1-1-1-implement-sse-identifier-and-replay-cursor-helpers.md):
    execution plan for validated SSE event identifiers and `Last-Event-ID`
    header parsing (task 1.1.1).
  - [Implement SSE frame and cache-header
    helpers](execplans/1-1-2-sse-frame-and-cache-header-helpers.md):
    execution plan for shared SSE frame rendering and live event-stream
    cache-control helpers (task 1.1.2).
  - [Implement shared heartbeat and `stream_reset`
    helpers](execplans/1-1-3-shared-heartbeat-and-stream-reset-helpers.md):
    execution plan for the shared 20-second heartbeat policy and standard
    replay-reset SSE helper (task 1.1.3).
