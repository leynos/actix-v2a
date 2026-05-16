# Developer's Guide

This guide documents internal conventions, module structure, and quality gates
for contributors working on the `actix-v2a` crate.

## SSE module internals

The `src/sse/` module implements validated Server-Sent Events (SSE) wire-level
helpers per
[ADR 001](adr-001-shared-sse-wire-contract-for-wildside-and-corbusier.md).

### Module layout

```plaintext
src/sse/
  mod.rs               # Public re-exports and module documentation
  actix_adapter.rs     # Actix Web adapter functions for SSE domain helpers
  cache_control.rs     # Live event-stream cache policy helper
  event_id.rs          # EventId validated newtype and validation error
  frame.rs             # SSE event and comment frame rendering
  header.rs            # Framework-agnostic SseHeader type
  heartbeat.rs         # Heartbeat interval policy and canonical heartbeat frame
  replay_cursor.rs     # ReplayCursor type and Last-Event-ID header extraction
  stream_reset.rs      # Standard replay-unavailable control event helper
```

### Validation strategy

#### Forbidden character detection

The `EventId` validation scans for exactly three forbidden byte values:

- Carriage return: `0x0D` (U+000D)
- Line feed: `0x0A` (U+000A)
- NULL: `0x00` (U+0000)

These are the characters that would corrupt the SSE wire format per the WHATWG
HTML specification § 9.2.6. No other characters are forbidden by the wire
format.

The validation uses byte iteration (not char iteration) for efficiency, since
all three forbidden values are single-byte characters in UTF-8:

```rust
for byte in value_str.as_bytes() {
    if matches!(byte, b'\r' | b'\n' | b'\0') {
        return Err(EventIdValidationError::ForbiddenCharacter);
    }
}
```

#### Empty identifier handling

Empty identifiers are rejected because an empty `id:` field has special meaning
in the SSE specification: it resets the last event identifier rather than
setting one. An empty `Last-Event-ID` header value is treated as `Ok(None)` by
`extract_replay_cursor`, consistent with this specification behaviour.

#### Whitespace preservation

Unlike `IdempotencyKey`, which trims whitespace and validates UUID format,
`EventId` preserves leading and trailing whitespace verbatim. The identifier is
treated as an opaque string where whitespace may be meaningful to the
identifier generation strategy.

### Type relationships

- `EventId` — validated newtype wrapping `String`. Construction validates
  absence of CR, LF, and NULL.
- `ReplayCursor` — validated newtype wrapping `EventId`. Distinguishes "outgoing
  identifier for SSE `id:` line" from "identifier received from client
  reconnection header".
- `EventIdValidationError` — validation error enum with two variants:
  - `Empty` — identifier was empty
  - `ForbiddenCharacter` — identifier contained CR, LF, or NULL
- `ReplayCursorError` — replay cursor extraction error enum:
  - `Empty` — replay cursor value was empty after validation
  - `ForbiddenCharacter` — replay cursor contained CR, LF, or NULL
  - `InvalidHeader` — `Last-Event-ID` header was malformed (duplicate or
    non-UTF-8)
- `SseFrameError` — event/comment frame rendering error enum:
  - `EmptyEventName` — an explicit event name was provided but empty
  - `InvalidEventName` — event name contained CR, LF, or NULL
  - `InvalidData` — event payload contained NULL
  - `InvalidComment` — comment payload contained NULL
- `HeartbeatPolicy` — typed heartbeat interval wrapper with a 20-second default
  and explicit non-zero override path
- `HeartbeatPolicyError` — heartbeat policy validation error enum:
  - `ZeroInterval` — explicit override used `Duration::ZERO`

The `ReplayCursor` wrapping is semantic, not functional. Both types expose the
same string via `as_ref()` and `Display`. The distinction allows type
signatures to communicate intent: an `EventId` parameter documents "this
function emits SSE frames", while a `ReplayCursor` parameter documents "this
function handles client reconnection".

### Header extraction pattern

The `extract_replay_cursor` function follows the same single-header-or-error
pattern used by `extract_idempotency_key` in `src/idempotency/http.rs`:

```rust
let mut header_values = headers.get_all(LAST_EVENT_ID_HEADER);
let Some(header_value) = header_values.next() else {
    return Ok(None);  // Missing header is allowed
};
if header_values.next().is_some() {
    return Err(ReplayCursorError::InvalidHeader);  // Duplicate header fails
}
```

Key differences from idempotency key extraction:

- Empty header values return `Ok(None)` (not an error) because the SSE
  specification treats empty `id:` fields as a reset of the last event
  identifier.
- Header values are not trimmed, consistent with treating identifiers as opaque
  strings.

### Frame rendering rules

The framing helpers stay deliberately small:

- `render_event_frame` renders complete frames with deterministic field order:
  `id:`, then `event:`, then one or more `data:` lines, then a blank line.
- `render_comment_frame` renders one or more comment lines followed by a blank
  line.
- `event_name: None` is the only supported way to express the default browser
  `message` event. `Some("")` is rejected.
- `data` and comment payloads normalize `\r`, `\n`, and `\r\n` into logical
  line breaks. This preserves embedded blank lines and trailing newlines while
  keeping the rendered wire text valid.
- NULL is rejected in `data` and comment payloads because it cannot be
  represented safely in the shared wire contract.

The implementation intentionally uses pure string rendering helpers rather than
an Actix responder, so downstream applications keep control of stream
lifecycle, heartbeats, authorization, and replay orchestration.

### Heartbeat and stream-reset helpers

Task 1.1.3 adds two higher-level helpers on top of `frame.rs`:

- `render_heartbeat_frame` delegates to `render_comment_frame("")` so the
  approved heartbeat frame stays aligned with the lower-level comment framing
  rules.
- `render_stream_reset_frame` delegates to `render_event_frame` with the fixed
  event name `stream_reset` and the fixed payload
  `{"reason":"replay_unavailable"}`.

The heartbeat policy remains data-only. `HeartbeatPolicy::default()` exposes
the ADR default of 20 seconds, while `HeartbeatPolicy::new` accepts explicit
non-zero overrides and rejects `Duration::ZERO`. This keeps "disable
heartbeats" outside the shared wire contract and avoids smuggling scheduler
policy into the crate.

### Cache-control policy

`apply_event_stream_cache_control` mutates a `Vec<SseHeader>` in place and sets:

- `Cache-Control: no-cache, no-store, must-revalidate`

For Actix Web callers, use `apply_actix_event_stream_cache_control` (in
`src/sse/actix_adapter.rs`), which accepts an Actix `HeaderMap` and delegates
to the domain function after converting header types.

### Framework adapter layer

The `actix_adapter` module is the sole location within `src/sse/` permitted to
import `actix_web` types. It exposes two public functions:

- `apply_actix_event_stream_cache_control(headers: &mut HeaderMap)` — inserts
  the canonical `Cache-Control` value into an Actix response `HeaderMap`.
<!-- markdownlint-disable-next-line MD013 -->
- `extract_actix_replay_cursor(headers: &HeaderMap) -> Result<Option<ReplayCursor>, ReplayCursorError>`
  — collects all `Last-Event-ID` values from an Actix request `HeaderMap`,
  converts each to a domain `SseHeader`, and delegates to
  `extract_replay_cursor`.

The `SseHeader` type (in `src/sse/header.rs`) is a plain name/value pair with
case-insensitive name comparison. Domain functions accept `&[SseHeader]` or
`&mut Vec<SseHeader>` to remain framework-independent.

### Error mapping

The `map_replay_cursor_error` function maps each `ReplayCursorError` variant to
an `ErrorCode::InvalidRequest` error with a descriptive message:

- `ReplayCursorError::Empty` → "last-event-id must not be empty"
- `ReplayCursorError::ForbiddenCharacter` → "last-event-id must not contain
  carriage return, line feed, or null"
- `ReplayCursorError::InvalidHeader` → "last-event-id header is malformed"

These messages are suitable for client-facing error responses and follow the
same pattern as `map_idempotency_key_error`.

## Build tooling

Netsuke is the primary repository build driver during the dogfooding pilot that
replaces direct GNU Make usage. Netsuke compiles the root `Netsukefile` into a
Ninja build graph, then Ninja runs the selected action. Install Ninja and
Netsuke before running repository gates:

```bash
sudo dnf install ninja-build
cargo install --git https://github.com/leynos/netsuke.git \
  --rev 2fe314a58d7311758640b3daa086c401d79838cf \
  netsuke --locked
```

Use the documented Netsuke targets instead of calling the underlying commands
directly. The root `Makefile` remains as a compatibility shim for existing
hooks and developer habits, but each Make target now delegates to Netsuke.

The Netsuke actions prepend `$HOME/.cargo/bin`, `$HOME/.bun/bin`, and
`$HOME/.local/bin` while preserving the caller-provided `PATH`. This keeps
targets working when hook environments omit common user install directories.
Cargo-based actions honour `CARGO`; Markdown and Mermaid actions honour
`MDLINT` and `NIXIE`.

Override Cargo by exporting `CARGO` before invoking Netsuke:

```bash
CARGO=/path/to/cargo netsuke build test
```

The `test` action detects `cargo-nextest` through Cargo. Install
`cargo-nextest` to `$HOME/.cargo/bin` to enable `netsuke build test` to use
nextest.

```bash
cargo install cargo-nextest
```

If `cargo-nextest` is absent, `netsuke build test` falls back to `cargo test`
and still runs doctests.

Markdown linting uses `MDLINT`, resolved through the same scoped prepend. In
the standard development environment, `markdownlint-cli2` resolves from
`$HOME/.bun/bin`:

```bash
netsuke build markdownlint
```

Developers using CI hooks, reduced-`PATH` shells, or other non-standard shell
environments must ensure the Cargo and Bun binary directories exist when those
tools are installed. The Netsuke manifest handles ordinary `$HOME/.cargo/bin`
and `$HOME/.bun/bin` installs automatically; it does not install missing tools
or create shims.

## Quality gates

All changes to the SSE module (and the broader crate) must pass the following
gates before commit:

### Formatting

```bash
netsuke build check-fmt
```

Validates that all Rust code is formatted per `rustfmt` conventions. Run
`netsuke build fmt` to apply formatting fixes.

### Lint

```bash
netsuke build lint
```

Executes Clippy with strict warnings (`-D warnings`) and the repository's
Whitaker custom lint suite. The Whitaker driver build can take up to 7 minutes
on first run; subsequent runs are fast.

Key enforced lints:

- `unwrap_used`, `expect_used` — no panics in production code
- `indexing_slicing` — bounds-checked access only
- `cognitive_complexity` — functions must be simple and clear
- `missing_const_for_fn` — mark eligible functions as `const`
- `missing_docs` — all public items require documentation

### Tests

```bash
netsuke build test
```

Runs `cargo nextest run` (if available, otherwise `cargo test`) plus
`cargo test --doc` for doctest coverage. All tests must pass. The SSE module
tests cover:

- Happy path validation for various identifier formats (ASCII, Unicode, emoji,
  numeric, UUID-formatted)
- Rejection of empty identifiers
- Rejection of identifiers containing forbidden characters (CR, LF, NULL) at
  start, middle, and end positions
- Preservation of leading and trailing whitespace
- Header extraction with missing, empty, duplicate, and non-UTF-8 headers
- Event frame rendering with optional `id:` and `event:` fields
- Data and comment newline normalization, including blank lines and trailing
  newlines
- Cache-control helper behaviour, replacement semantics, and determinism
- Property tests for `EventId` forbidden-byte validation and cache-control
  idempotence
- Heartbeat policy defaults, override validation, and heartbeat frame output
- Standard `stream_reset` event output and constant alignment
- Conversion traits (`AsRef<str>`, `Display`, `From`, `TryFrom`)
- Error mapping to API error envelope

### Testing dependencies

Test-only dependencies live under `[dev-dependencies]` in `Cargo.toml`.
Approved testing libraries are:

- `rstest` for fixtures and named parameterized examples.
- `rstest-bdd` and `rstest-bdd-macros` for downstream-facing behavioural
  scenarios where Gherkin structure clarifies the contract.
- `proptest` for compact invariants that should hold across generated input
  spaces, such as validation, parsing, normalization, and idempotence.
- `insta` for snapshot coverage of stable rendered output, such as error
  display text and OpenAPI schema JSON.
- `tracing-test` for assertions over emitted tracing spans and events.

### Documentation

```bash
netsuke build markdownlint  # Lint Markdown files
netsuke build nixie         # Validate Mermaid diagrams
```

Markdown files must pass `markdownlint` and any embedded Mermaid diagrams must
pass `nixie` validation.

## File size guidance

No single code file may exceed 400 lines. The SSE module respects this
constraint by splitting responsibilities across focused files:

- `event_id.rs` — identifier validation (currently ~250 lines including tests)
- `replay_cursor.rs` — cursor type and header extraction (currently ~290 lines
  including tests)
- `frame.rs` — event/comment rendering rules and regression tests
- `cache_control.rs` — cache policy helper and regression tests

If a file grows beyond 400 lines, extract helper functions to a new module file
or split behavioural test suites to a dedicated `tests/` subdirectory.

## Testing conventions

### Unit tests

Use `rstest` for parameterized test cases where multiple inputs share the same
assertion structure:

```rust
#[rstest]
#[case("\n")]
#[case("evt\n123")]
#[case("evt-123\n")]
fn new_rejects_identifier_containing_line_feed(#[case] input: &str) {
    let result = EventId::new(input);
    assert_eq!(result, Err(EventIdValidationError::ForbiddenCharacter));
}
```

Avoid underscore-prefixed label parameters in rstest cases (triggers
`used_underscore_binding` lint).

### Property tests

Use `proptest` when a helper has a compact invariant that should hold across a
large input space, especially for parsing, validation, normalization, and
idempotent mutation. Keep property tests beside the helper they exercise, in
the same module-local `#[cfg(test)] mod tests` block as the related
example-based coverage.

Prefer `rstest` for named edge cases that document specific examples. Prefer
`proptest` when the important claim is universal, such as "all UTF-8 strings
without forbidden bytes are valid" or "applying this header policy twice leaves
one canonical header". Property tests should:

- use narrow strategies that match the invariant under test;
- keep generated collection sizes bounded so `netsuke build test` remains fast;
- use `prop_assert!` and `prop_assert_eq!` for failures that shrink clearly;
- avoid returning `Result` from the generated test body unless the property
  genuinely needs fallible setup outside the assertion path; and
- preserve hand-written regression tests for named boundary cases that are easy
  for reviewers to read.

Current SSE examples are:

- `sse::event_id::tests::arbitrary_byte_validation_matches_spec`, which checks
  `EventId::try_from(String)` over arbitrary byte vectors converted to a
  `String` with lossy UTF-8 decoding (`String::from_utf8_lossy`). This does not
  filter out invalid-UTF-8 inputs before calling `EventId::try_from(String)`.
- `sse::cache_control::tests::apply_event_stream_cache_control_is_idempotent_over_arbitrary_headers`,
  which checks cache-control canonicalization over generated header vectors.

### Behavioural tests

Use `rstest-bdd` when the scenario structure adds clarity. The SSE module keeps
detailed edge-case coverage in module-local unit tests, then uses
`tests/sse_wire_contract_bdd.rs` and `tests/features/sse_wire_contract.feature`
for downstream-style scenarios that compose the public crate-root re-exports.
Keep this split: add precise validation permutations beside the helper being
tested, and add behavioural scenarios only when they prove the published wire
contract more clearly than a direct assertion test.

### Test organization

Module-local/unit tests live in `#[cfg(test)] mod tests` blocks within the
implementation file. Integration contract tests live under `tests/` and must
import from `actix_v2a`, not private module paths. Module-level tests use `//!`
comments to describe coverage scope:

```rust
#[cfg(test)]
mod tests {
    //! Regression coverage for SSE event identifier validation.

    use super::*;

    #[test]
    fn new_accepts_simple_ascii_identifier() {
        // ...
    }
}
```

## Next steps

The initial SSE wire-helper milestone from roadmap tasks 1.1.1 through 1.1.3 is
now complete:

- validated event identifiers and replay cursors;
- deterministic frame and cache-header helpers;
- typed heartbeat policy plus canonical heartbeat and `stream_reset` helpers.

See [`roadmap.md`](roadmap.md) for the full delivery plan.

## Pagination module internals

The `src/pagination/` module provides reusable cursor pagination primitives
without binding the crate to a database, repository, or route shape.

### Pagination module layout

- `src/pagination/mod.rs` — module documentation, public re-exports, and the
  documented pagination error-mapping table.
- `src/pagination/cursor.rs` — `Cursor<Key>` encoding and decoding, direction
  handling, token validation, and tracing spans.
- `src/pagination/params.rs` — `PageParams` parsing, limit normalisation, and
  shared query parameter constants.
- `src/pagination/envelope.rs` — `Paginated<T>` response envelopes and
  `PaginationLinks` link construction.

### Cursor ordering invariant

Cursor keys must implement `Serialize` and `Deserialize` with a consistent
representation that is compatible with the endpoint's total ordering. The
module encodes and decodes opaque cursor tokens, but it does not prove that a
database query, index, or repository predicate applies the same ordering on
every page. Downstream persistence logic owns that invariant.

### Limit normalisation

`PageParams` normalises page limits consistently:

- missing `limit` values use `DEFAULT_LIMIT`;
- limits greater than `MAX_LIMIT` are clamped to `MAX_LIMIT`;
- `limit=0` is rejected with `PageParamsError::InvalidLimit`.

### Tracing

`Cursor::encode` and `Cursor::decode` are instrumented with `#[instrument]`
spans. `CursorError::Serialize` additionally emits a `tracing::error!` event at
the public `Cursor::encode` boundary because it indicates that the server could
not serialise its own cursor key. The caller-controlled variants
`CursorError::InvalidBase64`, `CursorError::Deserialize`,
`CursorError::TokenTooLong`, and `PageParamsError::InvalidLimit` do not emit
library error events.

### Pagination error mapping

HTTP adapters should map caller-controlled pagination failures to HTTP 400 and
`ErrorCode::InvalidRequest`:

- `CursorError::InvalidBase64`
- `CursorError::Deserialize`
- `CursorError::TokenTooLong`
- `PageParamsError::InvalidLimit`

`CursorError::Serialize` should map to HTTP 500 and `ErrorCode::InternalError`
because it is a server-side serialization failure.

### Testing patterns

Pagination tests are split by contract:

- `tests/pagination_documentation_bdd.rs` validates documented invariants for
  limits, cursor errors, and display text.
- `tests/pagination_http_bdd.rs` verifies handler-level HTTP status and
  `ErrorCode` mappings.
- `tests/pagination_tracing_tests.rs` verifies cursor span and event
  observability.
- `tests/snapshots/` stores `insta` snapshots for error `Display` outputs and
  OpenAPI schema JSON.

## Additional resources

- [ADR 001: Shared SSE wire contract for Wildside and
  Corbusier](adr-001-shared-sse-wire-contract-for-wildside-and-corbusier.md) —
  normative specification for the SSE module
- [ExecPlan: Implement SSE identifier and replay cursor
  helpers](execplans/1-1-1-implement-sse-identifier-and-replay-cursor-helpers.md)
   — implementation plan for task 1.1.1
- [ExecPlan: Implement SSE frame and cache-header
  helpers](execplans/1-1-2-sse-frame-and-cache-header-helpers.md) —
  implementation plan for task 1.1.2
- [Port Wildside pagination documentation hardening
  execplan](execplans/portwildsidepagination.md) — records the port scope,
  constraints, surprises, and acceptance criteria for the pagination
  documentation hardening workstream
- [AGENTS.md](../AGENTS.md) — code style, testing, and commit conventions
