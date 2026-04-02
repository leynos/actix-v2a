# Import shared HTTP primitives from Wildside

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DRAFT

## Purpose / big picture

`actix-v2a` currently contains only a stub library entrypoint, while
`../wildside/backend` already has working implementations for cursor
pagination, idempotency helpers, a shared JSON error envelope, and OpenAPI
schema wrappers around that envelope. The goal of this plan is to extract the
reusable parts of those features into `actix-v2a` so this repository becomes
the common home for version 2a Actix components instead of each application
re-implementing them.

The canonical upstream project is `https://github.com/leynos/wildside`, but for
local development and source inspection the implementation source of truth for
this plan is `../wildside/backend`.

Success is observable in five ways:

1. `src/lib.rs` exposes reusable modules for pagination, idempotency, shared
   error handling, `utoipa` OpenAPI fragments, and any approved Server-Sent
   Events (SSE) helpers.
2. Tests copied or adapted from `../wildside/backend` prove cursor codec
   behaviour, limit validation, idempotency parsing and replay semantics, and
   error responder output.
3. The repository documentation contains this plan and an updated
   `docs/contents.md` entry so the work is discoverable.
4. If SSE support is implemented, this repository carries an ADR that defines a
   shared wire contract for both Wildside and Corbusier before code lands.
5. The implementation passes the repository gates recorded with `tee` logs:
   `make check-fmt`, `make lint`, `make test`, `make markdownlint`, and
   `make nixie`.

## Repository orientation

Today this repository is greenfield. `src/lib.rs` contains only a greeting
stub, there is no `docs/execplans/` directory, and there are no existing
modules for HTTP contracts, pagination, idempotency, or OpenAPI support.
Because of that, implementation work should favour clean extraction over
compatibility shims.

The Wildside source surface relevant to this plan is:

- Pagination primitives:
  `../wildside/backend/crates/pagination/src/lib.rs`,
  `../wildside/backend/crates/pagination/src/cursor.rs`,
  `../wildside/backend/crates/pagination/src/params.rs`, and
  `../wildside/backend/crates/pagination/src/envelope.rs`.
- Pagination tests:
  `../wildside/backend/crates/pagination/tests/pagination_bdd.rs` and
  `../wildside/backend/crates/pagination/tests/features/`.
- Idempotency HTTP helpers:
  `../wildside/backend/src/inbound/http/idempotency.rs`.
- Idempotency domain primitives:
  `../wildside/backend/src/domain/idempotency/mod.rs`,
  `../wildside/backend/src/domain/idempotency/key.rs`,
  `../wildside/backend/src/domain/idempotency/payload.rs`,
  `../wildside/backend/src/domain/idempotency/mutation_type.rs`, and
  `../wildside/backend/src/domain/idempotency/record.rs`.
- Shared error envelope and Actix mapping:
  `../wildside/backend/src/domain/error.rs` and
  `../wildside/backend/src/inbound/http/error.rs`.
- OpenAPI schema wrappers:
  `../wildside/backend/src/inbound/http/schemas.rs`,
  `../wildside/backend/src/doc.rs`, and
  `../wildside/backend/tests/openapi_schemas_bdd.rs`.

No dedicated SSE helper module has been confirmed in this Wildside checkout.
The only nearby source found during planning was WebSocket heartbeat handling
in `../wildside/backend/src/inbound/ws/session.rs`, which is useful as design
inspiration but is not itself an SSE helper that can be imported verbatim.

Corbusier already has documented SSE expectations in
`/data/leynos/Projects/corbusier.worktrees/plan-front-end-adoption` (canonical
upstream: `https://github.com/leynos/corbusier`), especially in
`docs/corbusier-api-design.md` where `Last-Event-ID`, replay-aware
reconnection, global event streams, and explicit event identifiers are all part
of the planned application contract.

## Constraints

- Treat `../wildside/backend` as the implementation source to extract from, not
  as a runtime dependency to couple against. The default execution path is to
  transplant or adapt reusable code into this crate rather than path-depending
  on the Wildside application crate.
- Preserve the repository’s lint posture. New modules must keep module-level
  `//!` documentation, public Rustdoc, and Clippy compatibility with the
  existing `Cargo.toml` lint configuration.
- Keep the imported contracts generic. Pagination, idempotency hashing, and
  the shared error payload must not depend on Wildside-specific route, user, or
  persistence types after extraction.
- Keep module files below the repository’s 400-line file-size guidance. If a
  direct port would exceed that, split by concern during extraction.
- Do not invent SSE behaviour. If `../wildside/backend` does not contain a
  reusable SSE helper contract, define the shared contract first in an ADR,
  then stop before implementation of the SSE module until that ADR is approved.
- Treat the Wildside `utoipa` schema wrappers as in scope for the first
  implementation pass. The reusable target is shared schema fragments and
  related tests, not Wildside's full application-specific `ApiDoc`
  registration.

## Tolerances (exception triggers)

- Scope: if a first pass requires more than 24 files or roughly 1,400 net
  lines of code, stop and re-scope into smaller milestones.
- Interfaces: if extraction requires changing the user-visible JSON field names
  established in Wildside (`code`, `message`, `traceId`, `details`, `replayed`,
  `cursor`, `limit`, `links`), stop and escalate.
- Dependencies: adding the minimum dependency set implied by the Wildside
  sources is allowed, but any dependency beyond those directly justified by the
  imported modules requires escalation.
- SSE source: if no authoritative SSE helper source is located in
  `../wildside/backend` during implementation, do not improvize a production
  contract. Draft or update the SSE ADR instead, then stop after recording the
  discovery.
- OpenAPI scope: if porting the shared `utoipa` fragments requires pulling in
  Wildside-specific endpoint registration or application DTOs, stop and narrow
  the extraction back to reusable schema fragments only.
- Iterations: if the same failing gate persists after three focused fix
  attempts, stop and document the blocker before continuing.

## Risks

- Risk: the Wildside pagination crate is already transport-neutral, but the
  error and idempotency helpers currently mix transport and domain concerns.
  Severity: medium. Likelihood: high. Mitigation: extract generic types first,
  then introduce Actix adapter modules that depend on them.
- Risk: SSE helper code may not actually exist in the referenced Wildside
  checkout. Severity: high. Likelihood: high. Mitigation: make SSE an explicit
  discovery milestone and treat a missing source implementation as a stop
  condition rather than silently inventing one.
- Risk: OpenAPI wrappers may drag `utoipa` into a crate that otherwise wants to
  stay framework-light. Severity: medium. Likelihood: medium. Mitigation:
  isolate the `utoipa` work in a dedicated module and port only reusable schema
  fragments, not Wildside's full document assembly.
- Risk: idempotency replay behaviour is distributed across Wildside domain
  services and tests, so extracting only header parsing would miss the real
  contract. Severity: medium. Likelihood: high. Mitigation: capture both the
  parsing helpers and the payload-hash plus replay/conflict response
  expectations in tests before calling the work done.

## Proposed target layout

Unless discoveries during implementation force a split into subcrates, build
the first extraction as one library crate with focused modules:

```text
src/
  lib.rs
  pagination/
    mod.rs
    cursor.rs
    params.rs
    envelope.rs
  idempotency/
    mod.rs
    key.rs
    payload.rs
    mutation_type.rs
    record.rs
    http.rs
  http/
    mod.rs
    error.rs
    cache_control.rs
    sse.rs            # only if a real SSE source contract is confirmed
  openapi/
    mod.rs
    schemas.rs
tests/
  pagination_bdd.rs
  openapi_schemas_bdd.rs
  features/
    pagination.feature
    direction_aware_cursors.feature
```

`src/lib.rs` should re-export the stable public surface so downstream
applications can depend on `actix_v2a::pagination::PageParams` and similar
paths without reaching into implementation details.

## Implementation plan

### Milestone 1: establish the crate surface and pagination foundation

Create the `src/pagination/` module tree by porting the Wildside pagination
crate into this repository. Copy the transport-neutral code from
`../wildside/backend/crates/pagination/src/` with only the adjustments needed
to satisfy this repository’s lint rules, file-size limits, and public API paths.

Port the pagination tests next. Start with the Wildside BDD coverage in
`../wildside/backend/crates/pagination/tests/pagination_bdd.rs` and the
associated `features/` files. If the BDD harness proves heavier than this crate
needs, keep the behavioural assertions but collapse them into standard
integration tests only after preserving the exact behaviours: opaque cursor
round-tripping, direction-aware cursor decoding, limit defaulting and clamping,
zero-limit rejection, and next/prev/self link generation that preserves
non-pagination query parameters.

At the end of this milestone, `src/lib.rs` should export the pagination module,
and `make test` should prove the pagination feature set independently of any
Actix or Wildside route code.

### Milestone 2: extract idempotency primitives and HTTP contract helpers

Create `src/idempotency/` from the Wildside domain and HTTP helpers. The
minimum useful extraction is:

- `IdempotencyKey` and its validation errors.
- `PayloadHash` plus canonical JSON hashing.
- Mutation discriminators and record/query result types needed to represent
  replay versus conflict.
- An HTTP helper module that parses the `Idempotency-Key` header and maps bad
  headers into the shared API error type from Milestone 3.

Do not import Wildside repository ports or service implementations into this
crate. This library should stop at contract helpers and reusable types. The
tests should still encode the real semantics from Wildside:

- a missing header returns `Ok(None)`;
- a valid UUID header returns a parsed key;
- malformed headers become invalid-request errors;
- semantically identical JSON payloads hash identically;
- different payloads with the same idempotency key can be represented as a
  conflict.

Where Wildside expresses replay behaviour via response payloads with
`replayed: true`, add tests here that prove the helper contracts make that
possible even if the full domain service remains application-specific.

### Milestone 3: extract the common API error envelope and Actix responders

Create the shared error payload from `../wildside/backend/src/domain/error.rs`
and the Actix mapping layer from
`../wildside/backend/src/inbound/http/error.rs`. Keep the split explicit:

- a transport-agnostic error payload type and code enum;
- an Actix adapter that maps codes to HTTP status codes, inserts any trace
  identifier header, redacts internal error messages, and serializes the JSON
  body.

Port the Wildside tests for status mapping, redaction, and trace identifier
handling, or reproduce them verbatim if the originals are inline unit tests.
This milestone is complete when downstream handlers could return
`Result<T, actix_v2a::Error>` and get the exact JSON envelope expected by
Wildside clients.

### Milestone 4: extract shared OpenAPI fragments

Add an `openapi` module that ports the shared `utoipa` schema wrappers from
Wildside:

- `ErrorCodeSchema`;
- `ErrorSchema`;
- any generic schema wrappers that are truly reusable across applications.

Do not port Wildside-specific endpoint registration from `src/doc.rs`; this
crate should export fragments, not a complete application document.

Copy or adapt the relevant tests from
`../wildside/backend/tests/openapi_schemas_bdd.rs` so they verify the fragment
schema names and JSON field constraints without depending on Wildside's full
route set.

### Milestone 5: resolve the SSE helper question explicitly

Because Wildside does not currently expose a reusable SSE helper and Corbusier
already documents replay-aware SSE expectations, begin the SSE work by writing
or confirming an ADR in this repository. The initial draft is expected to live
at `docs/adr-001-shared-sse-wire-contract-for-wildside-and-corbusier.md`.

Run one focused implementation-time discovery pass against
`../wildside/backend` for reusable SSE code. The planning investigation did not
confirm any helper module for event identifier formatting, replay cursor
extraction from `Last-Event-ID`, cache-control defaults for event streams, or
heartbeat policy formatting. Only WebSocket heartbeat logic was found.

If a true SSE helper source is found during implementation, extract it into
`src/http/sse.rs` with tests that prove:

- event identifiers use the agreed format;
- replay cursors can be derived from the inbound header state;
- event-stream responses set the correct cache headers; and
- heartbeat helper output matches the agreed policy.

If no such source exists, stop after documenting the discovery. Do not derive
SSE behaviour from the WebSocket implementation without explicit approval. In
that case, the ADR becomes the deliverable that unblocks a later,
source-independent implementation pass.

## Validation and evidence capture

Use the repository Makefile targets and capture each gate with `tee` to a
stable log file under `/tmp`. Run the gates sequentially, never in parallel.

Use this command pattern during implementation:

```bash
set -o pipefail && make check-fmt | tee /tmp/check-fmt-actix-v2a-import-components-from-wildside.out
set -o pipefail && make lint | tee /tmp/lint-actix-v2a-import-components-from-wildside.out
set -o pipefail && make test | tee /tmp/test-actix-v2a-import-components-from-wildside.out
set -o pipefail && make fmt | tee /tmp/fmt-actix-v2a-import-components-from-wildside.out
set -o pipefail && make markdownlint | tee /tmp/markdownlint-actix-v2a-import-components-from-wildside.out
set -o pipefail && make nixie | tee /tmp/nixie-actix-v2a-import-components-from-wildside.out
```

Expected evidence after a successful implementation run:

- `make check-fmt` reports no formatting drift.
- `make lint` completes without Clippy, Rustdoc, or Whitaker failures.
- `make test` passes the imported unit and integration tests.
- `make markdownlint` and `make nixie` pass after this plan and
  `docs/contents.md` are added.

For planning-only documentation changes, replay only the documentation gates
unless new Rust code is introduced. This branch currently has no imported
runtime code yet, so Rust gate failures caused solely by the stub crate or by
`cargo nextest` finding zero tests should be recorded, not used to block the
planning artifact itself.

## Approval gates

Implementation must not begin until the following gates are satisfied:

1. Plan approved: the user confirms that this draft captures the intended scope
   and tolerances.
2. Import strategy confirmed: extract code into `actix-v2a` rather than path
   depending on the Wildside application crate.
3. SSE source decision made: either confirm an authoritative SSE helper source
   or accept that the first pass will stop short of SSE extraction.

## Progress

- [x] 2026-04-02 08:16 BST: confirmed the branch name
  `import-components-from-wildside` and created the required execplan path.
- [x] 2026-04-02 08:16 BST: inspected `actix-v2a` and confirmed the repository
  is currently a stub crate with no existing shared API modules.
- [x] 2026-04-02 08:16 BST: inspected `../wildside/backend` and mapped the
  concrete pagination, idempotency, error, and OpenAPI source files relevant to
  extraction.
- [x] 2026-04-02 08:16 BST: recorded the canonical upstream note that Wildside
  is hosted at `https://github.com/leynos/wildside` while local development
  should inspect `../wildside/backend`.
- [x] 2026-04-02 08:24 BST: inspected Corbusier planning documents and
  confirmed they already require replay-aware SSE semantics with
  `Last-Event-ID`, making an ADR necessary if `actix-v2a` is to host shared SSE
  helpers.
- [x] 2026-04-02 08:29 BST: passed `make markdownlint` and `make nixie` for the
  planning artifacts. Rust gates are intentionally deferred for this doc-only
  turn until there is imported Rust code to validate.
- [x] 2026-04-02 08:35 BST: updated the plan to treat shared `utoipa` schema
  fragments as explicitly in scope for the first implementation pass.
- [x] 2026-04-02 08:38 BST: revalidated the revised plan with
  `make markdownlint` and `make nixie` after confirming `utoipa` schema
  fragments are in scope.
- [ ] Await explicit approval before implementation.
- [ ] During implementation, keep this section updated after each milestone and
  after every gate run.

## Surprises & Discoveries

- `actix-v2a` does not yet have a `docs/contents.md` index or any prior
  execution plans, so this planning task must establish both.
- Wildside’s pagination primitives are already isolated in a dedicated crate,
  which makes them the cleanest first extraction target.
- Wildside’s error and OpenAPI story is reusable, but the current `doc.rs`
  file is application-specific and should not be copied wholesale.
- No dedicated SSE helper module was discovered in the inspected Wildside
  checkout. The current plan therefore treats SSE as an explicit discovery gate
  rather than assumed source-backed work.
- Corbusier already treats SSE as part of its planned stable API surface, so
  cross-application SSE helpers need a deliberate contract even if Wildside
  cannot yet donate an implementation.

## Decision Log

- 2026-04-02 08:16 BST: this plan assumes extraction into local modules rather
  than adding a path dependency on the Wildside backend application crate,
  because `actix-v2a` is meant to become the shared component library.
- 2026-04-02 08:16 BST: the OpenAPI target is shared schema fragments only, not
  Wildside's full `ApiDoc` document assembly.
- 2026-04-02 08:35 BST: the user confirmed that `utoipa` schemata are in
  scope, so Milestone 4 now assumes shared schema fragment extraction rather
  than treating OpenAPI parity as an open decision.
- 2026-04-02 08:16 BST: the SSE item remains unresolved because the planning
  investigation did not find authoritative reusable SSE code in
  `../wildside/backend`.
- 2026-04-02 08:24 BST: because Corbusier already depends on replay-aware SSE
  semantics, the absence of reusable Wildside SSE code makes an ADR necessary
  rather than optional.

## Outcomes & Retrospective

This section is intentionally incomplete because implementation has not begun.
When the work is finished, replace this note with a short factual summary of
what shipped, which milestones were deferred, the final gate outcomes, and the
main lessons learned from the extraction.
