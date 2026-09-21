# Architectural decision record (ADR) 002: scoped mutation contracts

## Status

Accepted as a design boundary. Implementation remains outstanding.

## Date

2026-09-20.

## Context and problem statement

Wildside, Corbusier, and Mornington need scoped deduplication and explicit
mutation outcomes. Existing actix-v2a helpers parse keys and compare hashes but
cannot express reservation ownership or unresolved effects. Consumer evidence
and ownership rules are recorded in the
[shared mutation design](shared-mutation-contract-design.md).

## Decision drivers

- Reuse across HTTP requests and background hook execution.
- Explicit concurrency and crash-recovery guarantees.
- Independence from PostgreSQL, RouchDB, and consumer domain models.

## Requirements

### Functional requirements

Operation identity must distinguish authorization scopes, operations, targets,
and client keys. A changed payload under the same identity must conflict.
Completion must match the current owner; results must support references and
application-defined values as well as optional snapshots.

### Technical requirements

Core types must compose with consumer domain ports without Actix request
objects or database handles. A reusable conformance harness must distinguish
atomic commit guarantees from separate-reservation guarantees.

## Options considered

### Keep extraction limited to keys and response snapshots

This preserves a small surface but leaves repeated and inconsistent recovery
logic in every consumer. It cannot express the identified common behaviour.

### Share behavioural contracts and leave durability to adapters

This provides reusable states, identity, and tests while allowing consumers to
select transactions or atomic aggregates. It requires adapter evidence.

### Own a generic transaction and retry framework

This centralizes orchestration but cannot make arbitrary effects atomic and
would import application-specific storage and recovery policy.

## Decision outcome

Adopt shared behavioural contracts. Extend the existing idempotency feature
rather than introduce a generic repository or transaction framework. Keep
payload fingerprinting separate from operation uniqueness. Require conditional
completion and explicit uncertain outcomes. Preserve authorization checks in
consumers, including on replay.

## Goals and non-goals

The goal is a common executable contract with consumer-owned persistence.
Distributed transactions, exactly-once external effects, authorization policy,
and backend implementations are non-goals.

## Migration plan

Roadmap 3.1 establishes the types and transitions; 3.2 publishes the harness,
key extraction, mutation outcome mappings, telemetry, and adoption examples.
Generic shared HTTP helpers, including validation, error conversion, pagination
conversion, and correlation, remain in phase 4. Existing APIs remain available
until a compatibility migration is documented. Consumer adoption follows
implemented capabilities, not acceptance of this ADR.

## Known risks and limitations

A correct state model does not make an adapter durable. A separately stored
reservation cannot prove whether an interrupted external effect happened.
Legacy records without scope cannot be silently assigned new meaning.

## Architectural rationale

A behavioural contract shares the stable requirement while allowing each
consumer to use its own persistence boundary. This follows the existing SSE
approach of sharing reusable semantics without owning the application service.

## Outstanding decisions

The boundary is settled. Concrete API representation, recovery, and migration
choices remain proposed in [ADR 003](adr-003-mutation-recovery-and-replay.md).
