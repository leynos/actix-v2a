# Implement validated SSE event identifier and replay cursor helpers

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DRAFT

## Purpose / big picture

`actix-v2a` is the shared component library for version 2a Actix services.
[ADR 001][adr-001] defines a wire-only Server-Sent Events (SSE) contract that
both Wildside and Corbusier can adopt without surrendering control of their own
event stores or stream routing. This plan delivers the first concrete piece of
that contract: validated event identifiers for SSE `id:` lines and a replay
cursor type that parses the `Last-Event-ID` request header.

After this change, downstream services gain two capabilities:

1. Constructing validated, wire-safe SSE event identifiers that are guaranteed
   free of carriage return (`\r`), line feed (`\n`), and NULL (`\0`) — the
   three characters that would corrupt an SSE `id:` line.
2. Parsing the `Last-Event-ID` HTTP request header into a validated replay
   cursor that can drive reconnection logic without the downstream service
   reimplementing header extraction or character validation.

These helpers are transport-focused. They impose no opinion on how identifiers
are generated (UUIDs, monotonic counters, composite keys) or how a service
stores or queries its event history. The identifier type accepts any string
that passes validation; the replay cursor type wraps the same validated
identifier extracted from a request header.

Success is observable by running `make test` and seeing the new unit and
behavioural tests pass. The helpers are exported from the crate root, so a
downstream consumer can write `use actix_v2a::sse::{EventId, ReplayCursor}`.

[adr-001]: ../adr-001-shared-sse-wire-contract-for-wildside-and-corbusier.md

## Constraints

These are hard invariants. Violation requires escalation, not workarounds.

- The SSE module must not depend on any application-specific event-store
  schema, persistence model, or endpoint routing. ADR 001 explicitly excludes
  these concerns from the shared library.
- The identifier validation must reject exactly three byte values: carriage
  return (U+000D), line feed (U+000A), and NULL (U+0000). These are the
  characters that would break the SSE wire format per the
  [WHATWG HTML specification][whatwg-sse] § 9.2.6. No other characters are
  forbidden by the wire format.
- The `Last-Event-ID` header parsing must follow the same single-header
  extraction pattern used by the existing `extract_idempotency_key` function in
  `src/idempotency/http.rs`: missing headers are `Ok(None)`, duplicate headers
  are an error, non-UTF-8 values are an error.
- The identifier type must be independent of any particular identifier
  generation strategy. It accepts any validated string, not only UUIDs,
  integers, or any other specific format.
- Public interfaces must carry Rustdoc comments, examples, and satisfy the
  `missing_docs` deny lint. Module-level `//!` comments are required.
- All new code must pass the repository's strict Clippy configuration
  (30+ deny-level lints including `unwrap_used`, `expect_used`,
  `indexing_slicing`, `cognitive_complexity`, and `missing_const_for_fn`).
- File size must remain below the repository's 400-line guidance per file.
- en-GB-oxendict spelling in comments and documentation.
- Tests must use `rstest` fixtures and parameterized cases. Behavioural tests
  must use `rstest-bdd` where the scenario structure adds clarity.
- The `Last-Event-ID` header name constant must follow the
  [WHATWG HTML specification][whatwg-sse] capitalisation: `Last-Event-ID`.

[whatwg-sse]: https://html.spec.whatwg.org/multipage/server-sent-events.html

## Tolerances (exception triggers)

These thresholds define the boundaries of autonomous action. Breach triggers
escalation rather than improvisation.

- Scope: if implementation requires more than 6 new files or roughly 600 net
  lines of code, stop and re-scope.
- Interface: if a change to any existing public API signature is needed, stop
  and escalate.
- Dependencies: no new external crate dependencies are expected. If one becomes
  necessary, stop and escalate. All required dependencies (`actix-web`,
  `thiserror`, `serde`) are already in `Cargo.toml`.
- Iterations: if tests or lint still fail after three focused fix attempts on
  the same issue, stop and document the blocker.
- Ambiguity: if the ADR or WHATWG specification is ambiguous on a validation
  rule and the choice materially affects downstream behaviour, stop and present
  options.

## Risks

- Risk: the `Last-Event-ID` header value may contain characters that are valid
  per the SSE specification but unusual in practice (for example, non-ASCII
  Unicode or whitespace). The validation must not over-restrict: only CR, LF,
  and NULL are forbidden.
  Severity: medium. Likelihood: medium.
  Mitigation: test with diverse Unicode inputs including emoji, accented
  characters, and embedded spaces to confirm they pass validation. Only the
  three explicitly forbidden bytes are rejected.

- Risk: the WHATWG specification notes that the `Last-Event-ID` header value is
  set by the browser from the most recent `id:` field and may be an empty
  string when no `id:` has been seen. An empty header value is distinct from a
  missing header.
  Severity: low. Likelihood: high.
  Mitigation: treat an empty `Last-Event-ID` header value as `Ok(None)` (no
  cursor), consistent with the specification's intent that an empty `id:` field
  resets the last event identifier. Document this behaviour explicitly.

- Risk: the existing `extract_idempotency_key` function rejects surrounding
  whitespace because UUIDs have a defined format. The SSE identifier has no
  such constraint; leading or trailing whitespace in the `Last-Event-ID` header
  value should be preserved verbatim because the identifier is opaque.
  Severity: low. Likelihood: low.
  Mitigation: do not trim the header value. Document that the identifier is
  treated as an opaque string with only CR, LF, and NULL rejection. Test with
  identifiers containing leading and trailing spaces.

## Progress

- [ ] Read and internalise all referenced documents and existing code patterns.
- [ ] Write the execution plan draft.
- [ ] Receive plan approval.
- [ ] Create the `src/sse/` module directory with `mod.rs`.
- [ ] Implement `EventId` validated newtype in `src/sse/event_id.rs`.
- [ ] Implement `ReplayCursor` type and `Last-Event-ID` header parsing in
  `src/sse/replay_cursor.rs`.
- [ ] Wire the new `sse` module into `src/lib.rs` with public re-exports.
- [ ] Write unit tests for `EventId` (happy paths, forbidden characters,
  edge cases).
- [ ] Write unit tests for `ReplayCursor` and header extraction (happy paths,
  missing/duplicate/empty headers, forbidden characters).
- [ ] Write `rstest-bdd` behavioural tests where scenario structure adds
  clarity.
- [ ] Pass `make check-fmt`.
- [ ] Pass `make lint`.
- [ ] Pass `make test`.
- [ ] Update `docs/roadmap.md` to mark task 1.1.1 as done.
- [ ] Update `docs/contents.md` if new documents are added.
- [ ] Update `docs/developers-guide.md` with SSE module internal guidance.
- [ ] Update `docs/users-guide.md` with SSE identifier and replay cursor
  documentation.
- [ ] Pass `make markdownlint`.
- [ ] Pass `make nixie`.

## Surprises & discoveries

(No entries yet. This section will be updated as implementation proceeds.)

## Decision log

(No entries yet. This section will be updated as decisions are made during
implementation.)

## Outcomes & retrospective

(Not yet applicable. This section will be completed when the plan reaches
COMPLETE status.)

## Context and orientation

### Repository structure

`actix-v2a` is a single Rust library crate (edition 2024) at the repository
root. The source tree under `src/` is organized by feature:

```plaintext
src/
  lib.rs                 # crate root; re-exports the public surface
  error.rs               # transport-agnostic API error envelope
  http/
    mod.rs               # HTTP adapter module
    error.rs             # Actix ResponseError implementation
  idempotency/
    mod.rs               # idempotency module exports
    key.rs               # IdempotencyKey validated newtype
    payload.rs           # PayloadHash and canonical JSON hashing
    mutation_type.rs     # MutationType discriminator
    record.rs            # replay/conflict record types
    http.rs              # Last-Event-ID-style header extraction
  openapi/
    mod.rs               # OpenAPI schema fragment module
    schemas.rs           # utoipa schema wrappers
  pagination/
    mod.rs               # pagination module exports
    cursor.rs            # opaque cursor encoding/decoding
    params.rs            # query parameter validation
    envelope.rs          # paginated response envelope
```

### Key patterns to follow

The `IdempotencyKey` type in `src/idempotency/key.rs` is the closest
structural analogue to the new `EventId` type. Both are validated newtypes
wrapping a string value with construction-time validation. The key
differences are:

- `IdempotencyKey` validates that the string is a UUID. `EventId` validates
  that the string does not contain CR, LF, or NULL — a much simpler check.
- `IdempotencyKey` trims whitespace and rejects empty values. `EventId` must
  not trim whitespace (the identifier is opaque) but must reject empty values
  because an empty `id:` field has special meaning in the SSE specification
  (it resets the last event identifier rather than setting one).

The `extract_idempotency_key` function in `src/idempotency/http.rs` is the
structural analogue for `Last-Event-ID` header parsing. The same
single-header-or-error pattern applies. The key difference is that an empty
header value should be treated as `Ok(None)` rather than an error, because the
SSE specification assigns special meaning to an empty identifier.

### Governing documents

- [ADR 001][adr-001] defines the shared SSE wire contract. This plan
  implements the "validated event identifier type for `id:` lines and replay
  cursors" and "parsing helpers for the `Last-Event-ID` request header"
  items from the decision outcome.
- The [roadmap](../roadmap.md) tracks this work as task 1.1.1 under phase
  1.1 "Build the shared wire-helper surface".
- The [import-components-from-wildside execplan](import-components-from-wildside.md)
  is the parent execution plan. Milestone 5 deferred SSE implementation
  because no authoritative source existed in Wildside; ADR 001 now provides
  the normative contract.

### Build and validation commands

The repository uses a `Makefile` with these relevant targets:

- `make check-fmt` — verify formatting (`cargo fmt --all -- --check`).
- `make lint` — Clippy, Rustdoc, and Whitaker lints.
- `make test` — `cargo nextest run` (if available) plus `cargo test --doc`.
- `make fmt` — apply formatting fixes.
- `make markdownlint` — lint Markdown files.
- `make nixie` — validate Mermaid diagrams.

All gate commands should be run through `tee` with `set -o pipefail`:

```bash
set -o pipefail && make check-fmt 2>&1 | tee /tmp/check-fmt-sse-id.out
set -o pipefail && make lint 2>&1 | tee /tmp/lint-sse-id.out
set -o pipefail && make test 2>&1 | tee /tmp/test-sse-id.out
set -o pipefail && make markdownlint 2>&1 | tee /tmp/markdownlint-sse-id.out
set -o pipefail && make nixie 2>&1 | tee /tmp/nixie-sse-id.out
```

## Plan of work

The implementation proceeds in four stages. Each stage ends with a validation
step; the next stage must not begin until the current stage's validation
passes.

### Stage A: scaffold the SSE module and the `EventId` type

Create a new `src/sse/` module directory containing `mod.rs` and
`event_id.rs`. The `EventId` type is a validated newtype wrapping a `String`.
Construction rejects empty values and values containing CR, LF, or NULL.

In `src/sse/event_id.rs`, define:

- `EventIdValidationError` — a `thiserror` enum with variants `Empty` (the
  value was empty) and `ForbiddenCharacter` (the value contained CR, LF, or
  NULL). The `ForbiddenCharacter` variant should carry no payload because the
  exact offending character should not leak into error messages.
- `EventId` — a tuple struct wrapping `String` with:
  - `EventId::new(value: impl AsRef<str>)` returning
    `Result<Self, EventIdValidationError>` — validates and constructs.
  - `EventId::as_str(&self) -> &str` — accessor.
  - `impl AsRef<str>` — delegates to `as_str`.
  - `impl Display` — delegates to `as_str`.
  - `impl From<EventId> for String` — unwraps the inner value.
  - `impl TryFrom<String> for EventId` — delegates to `new`.

The validation function must check for the three forbidden bytes by iterating
over bytes (not chars) for efficiency, since CR, LF, and NULL are all
single-byte values in UTF-8.

In `src/sse/mod.rs`, re-export the public surface:

```rust
pub use event_id::{EventId, EventIdValidationError};
```

In `src/lib.rs`, add:

```rust
pub mod sse;
pub use sse::{EventId, EventIdValidationError};
```

Validation: `make check-fmt` and `make lint` pass. No tests yet.

### Stage B: add `ReplayCursor` and `Last-Event-ID` header parsing

Create `src/sse/replay_cursor.rs` containing:

- `LAST_EVENT_ID_HEADER` — a public `&str` constant with value
  `"Last-Event-ID"`.
- `ReplayCursor` — a tuple struct wrapping `EventId`. This type exists so
  downstream code can distinguish between "an identifier to attach to an
  outgoing SSE frame" and "an identifier received from a client's reconnection
  header". The inner `EventId` carries the same validation guarantees.
  - `ReplayCursor::new(event_id: EventId) -> Self` — wraps an already
    validated identifier.
  - `ReplayCursor::event_id(&self) -> &EventId` — accessor.
  - `ReplayCursor::into_event_id(self) -> EventId` — unwrap.
  - `impl AsRef<str>` — delegates through the inner `EventId`.
  - `impl Display` — delegates through the inner `EventId`.
- `extract_replay_cursor(headers: &HeaderMap)` returning
  `Result<Option<ReplayCursor>, EventIdValidationError>` — extracts
  and validates the `Last-Event-ID` header. Behaviour:
  - Missing header: `Ok(None)`.
  - Present but empty value: `Ok(None)` (the SSE specification treats an empty
    `id:` field as a reset of the last event identifier).
  - Present with a non-empty value containing forbidden characters:
    `Err(EventIdValidationError::ForbiddenCharacter)`.
  - Present with a valid non-empty value: `Ok(Some(ReplayCursor))`.
  - Duplicate headers: `Err(EventIdValidationError::InvalidHeader)`.
  - Non-UTF-8 header value: `Err(EventIdValidationError::InvalidHeader)`.

This means `EventIdValidationError` needs an additional variant:
`InvalidHeader` — for duplicate or non-UTF-8 header values. Add this variant
back in `src/sse/event_id.rs` during this stage.

Additionally, provide a convenience function for mapping validation errors to
the shared API error envelope:

- `map_replay_cursor_error(error: &EventIdValidationError) -> Error` — maps
  each variant to an `InvalidRequest` error with a descriptive message,
  following the same pattern as `map_idempotency_key_error` in
  `src/idempotency/http.rs`.

Update `src/sse/mod.rs` re-exports to include the new types:

```rust
pub use event_id::{EventId, EventIdValidationError};
pub use replay_cursor::{
    LAST_EVENT_ID_HEADER, ReplayCursor, extract_replay_cursor,
    map_replay_cursor_error,
};
```

Update `src/lib.rs` re-exports:

```rust
pub use sse::{
    LAST_EVENT_ID_HEADER, EventId, EventIdValidationError, ReplayCursor,
    extract_replay_cursor, map_replay_cursor_error,
};
```

Validation: `make check-fmt` and `make lint` pass.

### Stage C: write unit and behavioural tests

#### Unit tests for `EventId` (in `src/sse/event_id.rs`)

Use `rstest` parameterized tests where multiple cases share the same
assertion structure.

Happy path tests:

- Simple ASCII identifier (e.g. `"evt-001"`) succeeds.
- UUID-formatted identifier succeeds.
- Numeric string identifier succeeds.
- Identifier with embedded spaces succeeds (spaces are valid).
- Identifier with leading or trailing spaces succeeds (no trimming).
- Non-ASCII Unicode identifier (e.g. emoji, accented characters) succeeds.
- Single-character identifier succeeds.
- `as_str()` returns the exact input string.
- `Display` output matches `as_str()`.
- `From<EventId> for String` roundtrips.
- `TryFrom<String>` delegates to `new`.

Unhappy path tests:

- Empty string returns `EventIdValidationError::Empty`.
- String containing `\n` (line feed) returns `ForbiddenCharacter`.
- String containing `\r` (carriage return) returns `ForbiddenCharacter`.
- String containing `\0` (NULL) returns `ForbiddenCharacter`.
- String with forbidden character at start, middle, and end positions all
  return `ForbiddenCharacter` (use `rstest` parameterization).
- String containing only `\n` returns `ForbiddenCharacter` (not `Empty`,
  because the string is not empty — it contains a byte).

#### Unit tests for `ReplayCursor` (in `src/sse/replay_cursor.rs`)

Happy path tests:

- `extract_replay_cursor` with no `Last-Event-ID` header returns `Ok(None)`.
- `extract_replay_cursor` with a valid non-empty value returns
  `Ok(Some(cursor))` where `cursor.event_id().as_str()` matches the header
  value.
- `extract_replay_cursor` with an empty `Last-Event-ID` value returns
  `Ok(None)`.
- `ReplayCursor::new` wraps an `EventId` and `event_id()` returns it.
- `ReplayCursor::into_event_id` unwraps correctly.
- `AsRef<str>` on `ReplayCursor` returns the inner identifier string.
- Identifiers with leading/trailing spaces pass through without trimming.

Unhappy path tests:

- Duplicate `Last-Event-ID` headers return
  `Err(EventIdValidationError::InvalidHeader)`.
- Non-UTF-8 header value returns
  `Err(EventIdValidationError::InvalidHeader)`.
- Header value containing `\n` returns
  `Err(EventIdValidationError::ForbiddenCharacter)`.
- Header value containing `\r` returns
  `Err(EventIdValidationError::ForbiddenCharacter)`.
- Header value containing `\0` returns
  `Err(EventIdValidationError::ForbiddenCharacter)`.

Error mapping tests:

- `map_replay_cursor_error` produces `ErrorCode::InvalidRequest` for each
  variant.
- The constant `LAST_EVENT_ID_HEADER` equals `"Last-Event-ID"`.

Validation: `make check-fmt`, `make lint`, and `make test` all pass.

### Stage D: documentation, roadmap update, and cleanup

Update the following documents:

1. `docs/roadmap.md`: change the checkbox for task 1.1.1 from `[ ]` to `[x]`.

2. `docs/users-guide.md`: create this file (it does not yet exist) with
   content documenting the SSE event identifier and replay cursor types. Cover:
   - What `EventId` validates and why.
   - How to construct an `EventId` and handle validation errors.
   - What `ReplayCursor` represents and how to extract it from a request.
   - A concise code example showing construction and header extraction.

3. `docs/developers-guide.md`: create this file (it does not yet exist) with
   content documenting internal SSE module conventions. Cover:
   - The module layout under `src/sse/`.
   - The validation strategy (byte-level scan for forbidden characters).
   - The relationship between `EventId` and `ReplayCursor`.
   - How to run the quality gates.

4. `docs/contents.md`: add entries for any new documentation files.

5. ADR 001 design decisions addendum: if any design decisions were made during
   implementation that refine or extend the ADR, record them in the
   `Decision Log` section of this execplan and reference the ADR.

Validation: `make markdownlint` and `make nixie` pass on the final
documentation set. Then run the full Rust gate set one final time:
`make check-fmt`, `make lint`, `make test`.

## Concrete steps

All commands are run from the repository root `/home/user/project`.

### Stage A

1. Create `src/sse/event_id.rs` with the `EventId` type and
   `EventIdValidationError` enum as specified above.

2. Create `src/sse/mod.rs` with module declaration and re-exports.

3. Add `pub mod sse;` and the SSE re-exports to `src/lib.rs`.

4. Run:

   ```bash
   set -o pipefail && make check-fmt 2>&1 | tee /tmp/check-fmt-sse-id.out
   set -o pipefail && make lint 2>&1 | tee /tmp/lint-sse-id.out
   ```

   Expected: both pass with zero warnings.

### Stage B

1. Add the `InvalidHeader` variant to `EventIdValidationError`.

2. Create `src/sse/replay_cursor.rs` with `ReplayCursor`,
   `extract_replay_cursor`, `map_replay_cursor_error`, and the
   `LAST_EVENT_ID_HEADER` constant.

3. Update `src/sse/mod.rs` re-exports.

4. Update `src/lib.rs` re-exports.

5. Run:

   ```bash
   set -o pipefail && make check-fmt 2>&1 | tee /tmp/check-fmt-sse-id.out
   set -o pipefail && make lint 2>&1 | tee /tmp/lint-sse-id.out
   ```

   Expected: both pass.

### Stage C

1. Add `#[cfg(test)] mod tests` blocks to `src/sse/event_id.rs` and
   `src/sse/replay_cursor.rs` with the tests specified above.

2. Run:

   ```bash
   set -o pipefail && make check-fmt 2>&1 | tee /tmp/check-fmt-sse-id.out
   set -o pipefail && make lint 2>&1 | tee /tmp/lint-sse-id.out
   set -o pipefail && make test 2>&1 | tee /tmp/test-sse-id.out
   ```

   Expected: all pass. Inspect `/tmp/test-sse-id.out` for the new
   `sse::event_id::tests` and `sse::replay_cursor::tests` results.

### Stage D

1. Update `docs/roadmap.md`, create `docs/users-guide.md` and
   `docs/developers-guide.md`, and update `docs/contents.md`.

2. Run:

   ```bash
   set -o pipefail && make fmt 2>&1 | tee /tmp/fmt-sse-id.out
   set -o pipefail && make markdownlint 2>&1 | tee /tmp/markdownlint-sse-id.out
   set -o pipefail && make nixie 2>&1 | tee /tmp/nixie-sse-id.out
   ```

3. Run the full Rust gates one final time:

   ```bash
   set -o pipefail && make check-fmt 2>&1 | tee /tmp/check-fmt-sse-id.out
   set -o pipefail && make lint 2>&1 | tee /tmp/lint-sse-id.out
   set -o pipefail && make test 2>&1 | tee /tmp/test-sse-id.out
   ```

   Expected: all pass.

## Validation and acceptance

Quality criteria (what "done" means):

- Tests: `make test` passes with the new `sse::event_id::tests` and
  `sse::replay_cursor::tests` suites. All existing tests continue to pass.
- Lint/typecheck: `make check-fmt` and `make lint` report zero warnings.
- Documentation: `make markdownlint` and `make nixie` pass. Rustdoc builds
  cleanly (validated as part of `make lint`).
- Roadmap: task 1.1.1 is marked as done in `docs/roadmap.md`.
- Guides: `docs/users-guide.md` documents the SSE identifier and replay
  cursor behaviour. `docs/developers-guide.md` documents the module internals.

Quality method (how to check):

Run the validation commands listed in the concrete steps section. Compare
outputs against the expected results. Verify that the new test names appear in
the test output and all pass.

## Idempotence and recovery

All steps are idempotent. Creating files that already exist overwrites them
cleanly. Running `make fmt` before `make check-fmt` fixes any formatting
drift. Running `make test` is safe to repeat.

If a step fails partway through, fix the issue and re-run the stage's
validation commands. No rollback is needed because the changes are additive
and do not modify existing code.

## Artifacts and notes

The implementation produces these new files:

- `src/sse/mod.rs` — SSE module root.
- `src/sse/event_id.rs` — `EventId` validated newtype and
  `EventIdValidationError`.
- `src/sse/replay_cursor.rs` — `ReplayCursor`, header extraction, and error
  mapping.

The implementation modifies these existing files:

- `src/lib.rs` — adds `pub mod sse` and re-exports.
- `docs/roadmap.md` — marks task 1.1.1 as done.
- `docs/contents.md` — adds entries for new documentation files.

New documentation files:

- `docs/users-guide.md` — SSE identifier and replay cursor user-facing
  documentation.
- `docs/developers-guide.md` — SSE module internal guidance.

## Interfaces and dependencies

No new crate dependencies are required. The implementation uses:

- `actix-web` (already in `Cargo.toml`) — for `HeaderMap` in header
  extraction.
- `thiserror` (already in `Cargo.toml`) — for `EventIdValidationError`.
- `rstest` and `rstest-bdd` (already in dev-dependencies) — for tests.

### Public interface specification

In `src/sse/event_id.rs`:

```rust
/// Validation errors for [`EventId`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EventIdValidationError {
    /// The identifier value was empty.
    #[error("event identifier must not be empty")]
    Empty,
    /// The identifier value contained a forbidden character (CR, LF, or
    /// NULL).
    #[error(
        "event identifier must not contain carriage return, line feed, or null"
    )]
    ForbiddenCharacter,
    /// The HTTP header was malformed (duplicate or non-UTF-8).
    #[error("last-event-id header is malformed")]
    InvalidHeader,
}

/// Validated SSE event identifier for `id:` lines and replay cursors.
pub struct EventId(String);
```

In `src/sse/replay_cursor.rs`:

```rust
/// HTTP header name for the SSE reconnection identifier.
pub const LAST_EVENT_ID_HEADER: &str = "Last-Event-ID";

/// Replay cursor extracted from the `Last-Event-ID` request header.
pub struct ReplayCursor(EventId);

/// Extract the replay cursor from request headers.
pub fn extract_replay_cursor(
    headers: &HeaderMap,
) -> Result<Option<ReplayCursor>, EventIdValidationError>;

/// Map replay cursor validation failures to the shared API error
/// envelope.
pub fn map_replay_cursor_error(
    error: &EventIdValidationError,
) -> Error;
```
