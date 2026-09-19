# Architectural decision record (ADR) 003: mutation recovery and replay

## Status

Proposed. Resolve the decisions below before the dependent implementation tasks
are considered complete.

## Date

2026-09-20.

## Context and problem statement

Reservations survive requests, while effects and completion records may commit
separately. Wildside bundles and Corbusier hooks demonstrate this boundary.
Mornington additionally needs replay that respects redaction and avoids storing
one-time credentials as ordinary response snapshots.

## Decision drivers

- Safe behaviour after cancellation, crash, and ambiguous acknowledgement.
- Compatibility with existing keys, snapshots, and persisted records.
- A usable contract for both atomic and non-atomic effect boundaries.

## Requirements

### Functional requirements

Distinguish acquired, in-progress, completed, conflict, and indeterminate
outcomes. Support conditional completion, durable result references, and
consumer-controlled retention. Known terminal failures must remain distinct
from uncertain effects.

### Technical requirements

Inject time and bound waiting. Lease expiry alone must not trigger execution.
An old worker must not finalize a new owner's reservation. Persist fingerprint
profile versions and prove restart behaviour through adapter tests.

## Options considered

### Always cache and replay HTTP responses

This is simple but couples hooks to HTTP, retains stale resource bodies, and
cannot safely generalize one-time credential delivery.

### Share state and result contracts with consumer recovery

This accommodates references, typed results, and explicit snapshots. Consumers
must document how uncertain outcomes are reconciled and how authority is
rechecked before replay.

### Reclaim expired leases and retry automatically

This improves apparent availability but risks repeated effects while an old
worker still runs. A completion ownership check does not fence that worker's
external effect.

## Proposed direction

Use explicit state and result contracts. Prefer additive APIs and prohibit
implicit retries of unknown outcomes. Allow reclamation only with evidence that
execution cannot overlap or that the destination enforces fencing or
idempotency. Keep leases, result retention, and permanent deduplication markers
separate. The [design](shared-mutation-contract-design.md) specifies invariants
that any accepted representation must satisfy.

## Goals and non-goals

The goal is honest recovery and safe replay across storage models. Automatic
reconciliation of arbitrary effects and a universal retention duration are
non-goals.

## Migration plan

Resolve identity and fingerprint representation in 3.1.1, state and ownership
representation in 3.1.2, and recovery/result policy in 3.1.3. Record decisions
and alternatives here before marking those tasks complete. Prove them through
the downstream harness in 3.2.1.

## Outstanding decisions

- Should scope use generic domain wrappers or a validated opaque encoding?
  Compare Wildside user scope, Corbusier tenant scope, and Mornington actor and
  target scope; prove unambiguous encoding and isolation.
- Should the public surface expose a narrow store trait, transition functions,
  or both? Prototype atomic transaction and separate-reservation consumers
  without leaking a database handle or forcing non-atomic writes.
- Which ownership-token and persisted-state representation supports conditional
  completion and safe legacy migration? Define duplicate completion equality.
- How are result types versioned and old response snapshots migrated? Preserve
  existing records explicitly; define handling of missing fingerprint profiles.
- Which failures are retained, and how does a consumer signal reconciliation
  or safe reclamation? Demonstrate effect-before-completion crash recovery.
- How should callers inject clocks and finite waiting without a new scheduler
  framework? Define skew assumptions and cancellation behaviour.

## Known risks and limitations

These choices can affect persisted formats and public compatibility. A generic
API that forces separate commits would undermine atomic consumers. Retention
cleanup can reopen old identities; expiry therefore requires consumer policy.

## Architectural rationale

Resolve representation through consumer fixtures while holding the safety
invariants fixed. This avoids treating a convenient lease algorithm as proof of
exactly-once execution.
