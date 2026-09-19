# Roadmap

This roadmap records the next implementation workstreams for `actix-v2a`. Tasks
are ordered by dependency and written as measurable delivery units rather than
time-based commitments.

## 1. Shared SSE helpers from ADR 001

Implement the shared Server-Sent Events (SSE) wire-helper module defined by
[ADR 001](adr-001-shared-sse-wire-contract-for-wildside-and-corbusier.md)
without pulling event-store, routing, or authorization logic into this crate.

### 1.1. Build the shared wire-helper surface

- [x] 1.1.1. Implement validated SSE event identifier and replay cursor
  helpers. See ADR 001 "Decision outcome / proposed direction".
  - Reject carriage return, line feed, and NULL in identifier values.
  - Parse and validate the `Last-Event-ID` request header into the shared
    replay cursor type.
  - Keep the identifier helpers transport-focused and independent of any
    application-specific persistence model.
- [x] 1.1.2. Implement SSE frame and cache-header helpers. Requires 1.1.1. See
  ADR 001 "Functional requirements" and "Technical requirements".
  - Format `id:`, `event:`, `data:`, and comment heartbeat frames for
    event-stream responses.
  - Provide cache-control helpers that disable intermediary reuse of live event
    streams.
  - Exclude any convenience responder until both downstream applications prove
    the same lifecycle needs.
- [x] 1.1.3. Implement the shared heartbeat and `stream_reset` helpers.
  Requires 1.1.2. See ADR 001 "Decision outcome / proposed direction".
  - Provide the ADR default 20-second heartbeat policy with an explicit
    override path.
  - Emit the standard `stream_reset` event with payload
    `{"reason":"replay_unavailable"}`.
  - Keep application event names and payload schemata out of scope.

### 1.2. Prove the contract and close the delivery loop

- [x] 1.2.1. Add unit and integration tests for the shared SSE module.
  Requires 1.1.3. See ADR 001 "Migration plan" Phase 2.
  - Cover identifier validation, `Last-Event-ID` parsing, frame formatting,
    cache headers, heartbeat output, and `stream_reset` output.
  - Pass `make check-fmt`, `make lint`, and `make test` with the new coverage
    enabled.
- [x] 1.2.2. Update the execution plan and documentation after the SSE module
  lands. Requires 1.2.1.
  - Record the completed SSE milestone in
    `docs/execplans/import-components-from-wildside.md`.
  - Remove implementation-time environment-specific path references as required
    by the execplan closing milestone.
  - Pass `make fmt`, `make markdownlint`, and `make nixie` on the final
    documentation set.

## 2. Pagination documentation hardening

Harden the reusable `actix-v2a` pagination module with expanded documentation,
BDD and handler-level test coverage, and observability instrumentation.

- [x] 2.1. Port Wildside pagination documentation hardening.
  See
  [`docs/execplans/portwildsidepagination.md`](execplans/portwildsidepagination.md).
  - Documented cursor key ordering invariants and limit normalization semantics
    in `src/pagination/mod.rs`.
  - Added BDD coverage (`tests/pagination_documentation_bdd.rs`) and
    handler-level coverage (`tests/pagination_http_bdd.rs`).
  - Added snapshot tests for error `Display` outputs and OpenAPI schemas.
  - Instrumented `Cursor::encode` and `Cursor::decode` with tracing spans and
    error events.

## 3. Shared mutation contracts

Deliver the behavioural boundary in
[ADR 002](adr-002-scoped-mutation-contracts.md) and the
[mutation design](shared-mutation-contract-design.md). These unchecked tasks
specify future implementation; the design PR completes none of them.

### 3.1. Establish scoped reservation and replay contracts

Deliver a contract usable by HTTP mutations and non-HTTP hooks. Resolve API
choices against both consumers before committing to a storage interface.

- [ ] 3.1.1. Implement scoped identities and versioned request fingerprints.
  - Resolve ADR 003 identity representation and legacy-record migration.
  - Prove principal, tenant, operation, and target isolation, deterministic
    normalized hashing, profile upgrade handling, and payload conflicts.
  - Preserve existing UUID parsing and hashing APIs through explicit adapters.
  - See design sections "Scoped operation identity" and "Delivery and
    compatibility".
- [ ] 3.1.2. Implement reservation and conditional completion contracts.
  Requires 3.1.1.
  - Resolve ADR 003 state representation and completion ownership decisions.
  - Cover acquired, in-progress, completed, conflict, and indeterminate
    outcomes, duplicate completion, and rejection of stale owners.
  - Demonstrate both atomic commit and separate-reservation integration
    without exporting database transaction handles.
  - See design sections "Reservation and completion semantics" and
    "Atomicity and recovery".
- [ ] 3.1.3. Implement replay results and bounded recovery policy seams.
  Requires 3.1.2.
  - Resolve remaining ADR 003 questions and record its acceptance rationale.
  - Support resource references and typed results as well as opt-in snapshots.
  - Test injected time, cancellation, retention, and uncertain completion;
    prohibit automatic effect retries based solely on lease expiry.
  - See design sections "Replay and authorization" and "Atomicity and
    recovery".

### 3.2. Prove adapters and publish the mutation integration surface

Deliver a reusable harness that exposes differences between atomic adapters and
separately reserved effects before downstream adoption.

- [ ] 3.2.1. Publish a downstream-consumable mutation conformance harness.
  Requires 3.1.3.
  - Choose a test-support feature or companion package with rationale.
  - Cover every scenario in design section "Conformance and acceptance".
  - Include durable restart and ambiguous-acknowledgement fixtures; an
    in-memory reference implementation alone is insufficient evidence.
- [ ] 3.2.2. Implement required and optional key extraction and HTTP outcomes.
  Requires 3.1.3.
  - Resolve ADR 004 mutation status, reason, and retry-header mappings.
  - Test absent, malformed, and duplicate headers and key propagation into
    application dispatch; retain existing optional extraction compatibility.
  - Publish handler tests for replay, conflict, pending, and unknown outcomes.
- [ ] 3.2.3. Add bounded mutation telemetry and publish adoption examples.
  Requires 3.2.1, 3.2.2.
  - Emit outcome and latency instrumentation without high-cardinality labels
    or global recorder installation. See design "HTTP integration and other
    extractions".
  - Publish Wildside mutation and Corbusier task/hook integration examples,
    compatibility guidance, and a release or reviewed revision for adoption.
  - Link downstream issues and report which adapter guarantees their tests
    prove; do not claim downstream adoption from examples alone.

## 4. Shared HTTP integration helpers

Extract the bounded helpers in [ADR 004](adr-004-shared-http-integration.md)
without importing authentication, tenant policy, or persistence machinery.

### 4.1. Standardize validation and error conversion

Deliver consistent error details without breaking consumer response envelopes.
Use consumer compatibility fixtures to settle mapping decisions.

- [ ] 4.1.1. Implement structured field and index validation details.
  - Resolve ADR 004 validation vocabulary and envelope compatibility.
  - Cover nested fields, array indices, missing values, and safe omission of
    submitted secrets. Reuse Wildside validation cases.
  - See design "HTTP integration and other extractions".
- [ ] 4.1.2. Expose shared error and pagination HTTP conversion helpers.
  Requires 4.1.1.
  - Preserve explicit status, reason, safe details, and trace context.
  - Replace duplicated mapping in representative Corbusier fixtures and cover
    Wildside invalid-cursor and unsupported-direction cases.
  - Preserve existing pagination encoding and error APIs.

### 4.2. Standardize correlation and complete consumer migration guidance

Deliver request correlation independently of authentication and tracing policy,
then publish the complete adoption checklist.

- [ ] 4.2.1. Implement validated request-correlation propagation.
  - Resolve ADR 004 header trust, size, syntax, and invalid-input decisions.
  - Test read-or-create behaviour, consistent response/error propagation,
    concurrent request isolation, and distinction from distributed trace IDs.
- [ ] 4.2.2. Publish compatibility fixtures and downstream migration guidance.
  Requires 3.2.3, 4.1.2, 4.2.1.
  - Resolve ADR 004 outstanding decisions and record acceptance evidence.
  - Document public API migration, safe telemetry, and consumer-owned policy.
  - Update the users' and developers' guides and both downstream adoption
    issues with implemented task IDs and the adoption revision.
  - Pass formatting, lint, test, Markdown, and diagram gates.
