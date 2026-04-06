# User's Guide

This guide provides usage documentation for the `actix-v2a` crate's public API.
It assumes familiarity with Rust, Actix Web, and HTTP fundamentals.

## Server-Sent Events (SSE) helpers

The `sse` module provides wire-level helpers for implementing Server-Sent
Events endpoints without imposing opinions on event store design or identifier
generation strategies.

### Event identifiers (`EventId`)

The `EventId` type wraps a validated string that is safe to emit in an SSE
`id:` line. Construction rejects three characters that would corrupt the SSE
wire format per the WHATWG HTML specification § 9.2.6:

- Carriage return (U+000D)
- Line feed (U+000A)
- NULL (U+0000)

All other characters, including non-ASCII Unicode and whitespace, are accepted.
The identifier is treated as an opaque string with no trimming or format
requirements (UUID, integer, composite key, etc.).

#### Usage

```rust
use actix_v2a::EventId;

// Construct a validated identifier
let id = EventId::new("evt-001")?;

// Access the identifier string
assert_eq!(id.as_str(), "evt-001");

// Convert to String if needed
let id_string: String = id.into();
```

#### Error handling

Construction returns `EventIdValidationError`:

- `Empty` — the identifier value was empty.
- `ForbiddenCharacter` — the identifier contained CR, LF, or NULL.

### Replay cursors (`ReplayCursor`)

The `ReplayCursor` type wraps a validated `EventId` to distinguish between "an
identifier to attach to an outgoing SSE frame" and "an identifier received from
a client's reconnection header". The inner `EventId` carries the same
validation guarantees.

#### Extracting the `Last-Event-ID` header

Use `extract_replay_cursor` to parse and validate the `Last-Event-ID` request
header:

```rust
use actix_v2a::extract_replay_cursor;
use actix_web::HttpRequest;

fn handle_sse_request(req: HttpRequest) -> Result<(), Error> {
    let replay_cursor = extract_replay_cursor(req.headers())?;

    if let Some(cursor) = replay_cursor {
        // Client is reconnecting; start replay from cursor.event_id()
        let last_id = cursor.event_id();
        // ... resume stream from last_id
    } else {
        // New connection; start from beginning or latest event
        // ...
    }

    Ok(())
}
```

#### Header extraction behaviour

- Missing `Last-Event-ID` header: `Ok(None)`
- Empty `Last-Event-ID` value: `Ok(None)` (consistent with WHATWG specification
  treatment of empty `id:` fields as a reset)
- Valid non-empty value: `Ok(Some(ReplayCursor))`
- Duplicate headers: `Err(EventIdValidationError::InvalidHeader)`
- Non-UTF-8 value: `Err(EventIdValidationError::InvalidHeader)`
- Forbidden characters (CR, LF, NULL):
  `Err(EventIdValidationError::ForbiddenCharacter)`

#### Error mapping

Use `map_replay_cursor_error` to convert validation failures to the shared API
error envelope:

```rust
use actix_v2a::{extract_replay_cursor, map_replay_cursor_error};
use actix_web::HttpRequest;

fn handle_sse_request(req: HttpRequest) -> Result<(), actix_v2a::Error> {
    let replay_cursor = extract_replay_cursor(req.headers())
        .map_err(|e| map_replay_cursor_error(&e))?;

    // ... use replay_cursor
    Ok(())
}
```

All validation errors map to `ErrorCode::InvalidRequest` with descriptive
messages suitable for client responses.

### Header constant

The `LAST_EVENT_ID_HEADER` constant provides the standardized header name:

```rust
use actix_v2a::LAST_EVENT_ID_HEADER;

assert_eq!(LAST_EVENT_ID_HEADER, "Last-Event-ID");
```

## Identifier generation strategies

The SSE helpers do not prescribe how identifiers are generated. Downstream
services may use:

- UUIDs for globally unique identifiers across distributed systems.
- Monotonic counters or sequence numbers for ordered event streams.
- Composite keys encoding timestamp, partition, or offset information.
- Any other scheme that produces string values free of CR, LF, and NULL.

The `EventId` type validates the wire safety of the identifier, not its format
or generation strategy.

## Integration notes

- The SSE helpers are transport-focused and independent of event store
  persistence models, routing logic, or authorization policies.
- Event identifiers are preserved verbatim (no trimming) to support opaque
  schemes where whitespace may be meaningful.
- Validation occurs at construction time; once an `EventId` is created, it is
  guaranteed wire-safe.
- The `Last-Event-ID` header extraction follows the same single-header-or-error
  pattern used by the `extract_idempotency_key` function in the idempotency
  module.

For implementation guidance and internal conventions, see
[`developers-guide.md`](developers-guide.md).
