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
  cache_control.rs     # Live event-stream cache policy helper
  event_id.rs          # EventId validated newtype and validation error
  frame.rs             # SSE event and comment frame rendering
  replay_cursor.rs     # ReplayCursor type and Last-Event-ID header extraction
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
  - `InvalidHeader` — `Last-Event-ID` header was malformed (duplicate or
    non-UTF-8)
- `SseFrameError` — event/comment frame rendering error enum:
  - `EmptyEventName` — an explicit event name was provided but empty
  - `InvalidEventName` — event name contained CR, LF, or NULL
  - `InvalidData` — event payload contained NULL
  - `InvalidComment` — comment payload contained NULL

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
an Actix responder so downstream applications keep control of stream lifecycle,
heartbeats, authorization, and replay orchestration.

### Cache-control policy

`apply_event_stream_cache_control` mutates an Actix `HeaderMap` in place and
sets:

- `Cache-Control: no-cache, no-store, must-revalidate`

The helper replaces any prior `Cache-Control` state deterministically and does
not add proxy-vendor-specific buffering headers.

### Error mapping

The `map_replay_cursor_error` function maps each `ReplayCursorError` variant to
an `ErrorCode::InvalidRequest` error with a descriptive message:

- `ReplayCursorError::Empty` → "last-event-id must not be empty"
- `ReplayCursorError::ForbiddenCharacter` → "last-event-id must not contain
  carriage return, line feed, or null"
- `ReplayCursorError::InvalidHeader` → "last-event-id header is malformed"

These messages are suitable for client-facing error responses and follow the
same pattern as `map_idempotency_key_error`.

## Quality gates

All changes to the SSE module (and the broader crate) must pass the following
gates before commit:

### Formatting

```bash
make check-fmt
```

Validates that all Rust code is formatted per `rustfmt` conventions. Run
`make fmt` to apply formatting fixes.

### Lint

```bash
make lint
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
make test
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
- Conversion traits (`AsRef<str>`, `Display`, `From`, `TryFrom`)
- Error mapping to API error envelope

### Documentation

```bash
make markdownlint  # Lint Markdown files
make nixie         # Validate Mermaid diagrams
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

### Behavioural tests

Use `rstest-bdd` when the scenario structure adds clarity. The SSE module
currently uses only unit tests because the validation and rendering logic is
deterministic string and header formatting that does not benefit from
Given/When/Then structure.

### Test organization

Tests live in `#[cfg(test)] mod tests` blocks within the implementation file.
Module-level tests use `//!` comments to describe coverage scope:

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

After completing the SSE identifier, replay cursor, frame, and cache-header
implementation (tasks 1.1.1 and 1.1.2), the next roadmap task is:

- **1.1.3** — Heartbeat and `stream_reset` helpers (20-second heartbeat policy,
  `replay_unavailable` event)

See [`roadmap.md`](roadmap.md) for the full delivery plan.

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
- [AGENTS.md](../AGENTS.md) — code style, testing, and commit conventions
