# Roadmap

This roadmap records the next implementation workstreams for `actix-v2a`. Tasks
are ordered by dependency and written as measurable delivery units rather than
time-based commitments.

## 1. Shared SSE helpers from ADR 001

Implement the shared Server-Sent Events (SSE) wire-helper module defined by
[ADR 001](adr-001-shared-sse-wire-contract-for-wildside-and-corbusier.md)
without pulling event-store, routing, or authorization logic into this crate.

### 1.1. Build the shared wire-helper surface

- [ ] 1.1.1. Implement validated SSE event identifier and replay cursor
  helpers. See ADR 001 "Decision outcome / proposed direction".
  - Reject carriage return, line feed, and NULL in identifier values.
  - Parse and validate the `Last-Event-ID` request header into the shared
    replay cursor type.
  - Keep the identifier helpers transport-focused and independent of any
    application-specific persistence model.
- [ ] 1.1.2. Implement SSE frame and cache-header helpers. Requires 1.1.1. See
  ADR 001 "Functional requirements" and "Technical requirements".
  - Format `id:`, `event:`, `data:`, and comment heartbeat frames for
    event-stream responses.
  - Provide cache-control helpers that disable intermediary reuse of live event
    streams.
  - Exclude any convenience responder until both downstream applications prove
    the same lifecycle needs.
- [ ] 1.1.3. Implement the shared heartbeat and `stream_reset` helpers.
  Requires 1.1.2. See ADR 001 "Decision outcome / proposed direction".
  - Provide the ADR default 20-second heartbeat policy with an explicit
    override path.
  - Emit the standard `stream_reset` event with payload
    `{"reason":"replay_unavailable"}`.
  - Keep application event names and payload schemata out of scope.

### 1.2. Prove the contract and close the delivery loop

- [ ] 1.2.1. Add unit and integration tests for the shared SSE module.
  Requires 1.1.3. See ADR 001 "Migration plan" Phase 2.
  - Cover identifier validation, `Last-Event-ID` parsing, frame formatting,
    cache headers, heartbeat output, and `stream_reset` output.
  - Pass `make check-fmt`, `make lint`, and `make test` with the new coverage
    enabled.
- [ ] 1.2.2. Update the execution plan and documentation after the SSE module
  lands. Requires 1.2.1.
  - Record the completed SSE milestone in
    `docs/execplans/import-components-from-wildside.md`.
  - Remove implementation-time environment-specific path references as required
    by the execplan closing milestone.
  - Pass `make fmt`, `make markdownlint`, and `make nixie` on the final
    documentation set.
