# Architectural decision record (ADR) 001: shared SSE wire contract for Wildside and Corbusier

## Status

Proposed.

## Date

2026-04-02.

## Context and problem statement

`actix-v2a` is being positioned as the shared home for reusable version 2a
Actix components. During planning for the Wildside import, reusable source was
confirmed for pagination, idempotency, the common API error envelope, and
OpenAPI wrappers in `../wildside/backend` (canonical upstream:
`https://github.com/leynos/wildside`). The same planning pass did not find a
reusable Server-Sent Events (SSE) helper module in that checkout.

At the same time, Corbusier already has explicit SSE expectations in
`/data/leynos/Projects/corbusier.worktrees/plan-front-end-adoption` (canonical
upstream: `https://github.com/leynos/corbusier`). Its API design documents
require stable event identifiers, reconnection through the `Last-Event-ID`
header, replay-aware semantics, and event-stream endpoints that work with
browser `EventSource` clients.

If `actix-v2a` is going to provide SSE support for both Wildside and Corbusier,
it needs a shared wire contract that is generic enough for both applications,
specific enough to be testable, and small enough to avoid dragging
application-specific event stores or transport assumptions into this library.

## Decision Drivers

- Cross-application reuse for Wildside and Corbusier
- Browser `EventSource` compatibility
- Replay semantics that survive reconnects
- Clear separation between wire helpers and application event storage
- Transport-layer testability without full app stacks
- Predictable cache and heartbeat behaviour

## Requirements

### Functional requirements

- The shared library must format valid SSE frames for event-stream responses.
- The shared library must parse and validate the `Last-Event-ID` request
  header.
- The shared library must expose helpers for stable event identifiers and
  replay cursors without forcing one application-specific persistence model.
- The shared library must support replay-aware recovery, including a standard
  `stream_reset` event when an old event identifier can no longer be resumed.
- The shared library must provide a heartbeat helper that keeps idle
  connections alive without inventing fake domain events.

### Technical requirements

- The shared contract must remain transport-focused and must not depend on one
  application’s event-store schema.
- The wire helpers must be usable from `actix-web` handlers without requiring
  an entire application document or runtime.
- Event identifiers must reject carriage return, line feed, and NULL so the
  generated `id:` lines stay valid.
- Cache headers must disable intermediary reuse of live event streams.
- The heartbeat policy must be configurable, but the library should provide one
  sensible default.

## Options considered

### Option A: shared wire-only SSE helpers in `actix-v2a`

`actix-v2a` would expose validated event identifiers, `Last-Event-ID` parsing,
SSE frame formatting, stream cache headers, heartbeat helpers, and a small
`stream_reset` helper. Wildside and Corbusier would keep their own event-store,
auth, and projection logic.

### Option B: application-specific SSE implementations

Wildside and Corbusier would each implement their own SSE wire helpers and
replay semantics, with `actix-v2a` providing no shared streaming contract.

### Option C: shared full replay service in `actix-v2a`

`actix-v2a` would attempt to own not only wire helpers, but also event-store
interfaces, replay retention policies, and stream orchestration behaviour for
all applications.

| Topic                     | Option A | Option B                    | Option C               |
| ------------------------- | -------- | --------------------------- | ---------------------- |
| Cross-app consistency     | Strong   | Weak                        | Strong                 |
| Scope control             | Strong   | Strong per app, weak shared | Weak                   |
| Event-store coupling      | Low      | Low                         | High                   |
| Implementation complexity | Moderate | Moderate twice              | High                   |
| Reuse value               | High     | Low                         | High, but over-coupled |
| Fit for `actix-v2a`       | Strong   | Weak                        | Weak                   |

_Table 1: Trade-offs for the shared SSE contract._

## Decision outcome / proposed direction

`actix-v2a` should provide a shared, wire-only SSE module that both Wildside
and Corbusier can adopt without surrendering control of their own event stores
or stream routing.

Under this direction, the shared SSE surface should include:

- a validated event identifier type for `id:` lines and replay cursors;
- parsing helpers for the `Last-Event-ID` request header;
- SSE frame formatting helpers for `id:`, `event:`, `data:`, and comment
  heartbeat frames;
- cache header helpers for event-stream responses;
- a heartbeat policy helper with a default interval and explicit override path;
- a standard `stream_reset` helper for the case where replay cannot resume from
  the supplied identifier.

The shared module should not own:

- authentication policy;
- event-store persistence or retention;
- application-specific event names or payload schemas;
- endpoint routing such as `/api/v1/events` versus
  `/api/v1/events/conversations/{id}`.

## Goals and non-goals

### Goals

- Give Wildside and Corbusier one shared SSE wire contract.
- Keep replay semantics explicit and browser-compatible.
- Make the shared helpers small enough to live comfortably in `actix-v2a`.

### Non-goals

- Standardize one event-store schema for both applications.
- Force one event identifier generation algorithm on every application.
- Replace application-specific stream routing and authorization rules.

## Migration plan

### Phase 1

Land this ADR and update the Wildside import execplan so the SSE work is
blocked on an explicit contract rather than assumed source parity.

### Phase 2

Implement the shared SSE module in `actix-v2a` with unit and integration tests
covering identifier validation, `Last-Event-ID` parsing, frame formatting,
cache headers, heartbeat comments, and `stream_reset` output.

### Phase 3

Adopt the shared helpers in Wildside and Corbusier adapters while leaving their
event-store and endpoint orchestration code in their own repositories.

## Known risks and limitations

- Wildside and Corbusier may still diverge on retention windows and replay
  storage; this ADR only standardizes the wire boundary.
- If one application requires binary or multi-line payload framing beyond the
  chosen helper surface, the shared abstraction may need careful extension.
- A shared default heartbeat interval will always be a compromise across
  deployments and proxy topologies.

## Outstanding decisions

- Whether the default heartbeat interval should be 15 seconds, 20 seconds, or
  another value proven safe for both deployments
- Whether `stream_reset` should be a bare event with an empty JSON object or a
  structured payload that names the replay failure reason
- Whether `actix-v2a` should expose a convenience Actix responder alongside the
  lower-level wire helpers in its first pass

## Architectural rationale

This direction keeps `actix-v2a` focused on reusable transport components
instead of becoming a shadow application framework. It also matches Corbusier’s
published SSE expectations around `Last-Event-ID`, replay, and stable event
identifiers while acknowledging that Wildside does not yet provide a directly
extractable SSE helper. A shared wire contract is therefore the narrowest
stable seam that helps both applications without overfitting either one.
