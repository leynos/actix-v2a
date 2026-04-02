# Documentation contents

- [Documentation contents](contents.md): entry point for the documentation
  set, including guides, references, and execution plans.
- [Documentation style guide](documentation-style-guide.md): repository
  standards for structure, tone, formatting, and maintenance of documentation.
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
  - [Import components from Wildside](execplans/import-components-from-wildside.md):
    draft plan for extracting shared HTTP and API primitives from
    `../wildside/backend` into this crate.
