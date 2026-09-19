# Architectural decision record (ADR) 004: shared HTTP integration

## Status

Proposed. Existing wire behaviour remains unchanged until implementation and
compatibility fixtures establish the chosen mappings.

## Date

2026-09-20.

## Context and problem statement

Wildside carries field/index validation and pagination error helpers. Corbusier
duplicates shared error conversion and maintains request correlation. Both need
mutation outcomes that callers can interpret consistently. Extraction must
preserve consumer envelopes and avoid importing authorization policy.

## Decision drivers

- Reuse of existing actix-v2a errors, pagination, and key parsing.
- Stable machine-readable details and explicit retry semantics.
- Safe diagnostics and bounded telemetry cardinality.

## Requirements

### Functional requirements

Provide distinct required and optional key extraction. Define HTTP responses
for pending, completed, conflicting, and uncertain operations. Provide field
paths, optional indices, safe reason details, and consistent correlation.

### Technical requirements

Preserve explicit HTTP status and trace context. Do not reflect submitted
secrets by default. Correlation identifiers must have validated syntax and
length and must not establish trust or impersonate distributed trace IDs.
Metrics must not use raw or hashed principal identifiers as labels.

## Options considered

### Copy consumer helpers unchanged

This is quick but retains differing error shapes and policies. Wildside's
user-derived metric labels remain high-cardinality even when hashed.

### Extend existing feature surfaces with compatibility fixtures

This shares conversion and validation behaviour while retaining application
messages, authorization, telemetry installation, and storage policy locally.

### Introduce universal HTTP middleware

This could automate dispatch but would obscure transaction boundaries and force
unrelated consumers into the same response and authentication lifecycle.

## Proposed direction

Extend existing features with narrowly scoped helpers and handler fixtures.
Keep mandatory-key policy explicit per endpoint and carry parsed keys into
application services. Use generic correlation propagation only after its trust
policy is settled. Defer protected cursor codecs and new SSE responders until
separate consumer requirements justify them.

## Goals and non-goals

The goal is consistent, composable HTTP integration. Authentication middleware,
session policy, business error classification, and a new pagination engine are
non-goals.

## Migration plan

Task 3.2.2 resolves mutation wire mappings. Tasks 4.1.1 and 4.1.2 resolve
validation details and conversion. Task 4.2.1 resolves correlation. Task 4.2.2
records acceptance evidence and publishes consumer migrations. Telemetry in
3.2.3 uses bounded categories and leaves recorder installation to applications.

## Outstanding decisions

- Which status codes, stable reasons, and retry headers distinguish pending
  and indeterminate outcomes without implying a safe retry? Test both consumer
  clients and document whether retry means lookup or attempted execution.
- How should common reason, field, and index details fit existing envelopes?
  Compare serialized compatibility fixtures before choosing field names.
- Which correlation header, limits, and syntax apply? Decide whether invalid
  or untrusted input is rejected or replaced, and define propagation rules
  across successful responses, errors, and inbound tracing context.
- Which public error conversions preserve consumer-selected status while
  preventing sensitive internal details from escaping? Avoid silently mapping
  unsupported statuses to misleading shared error categories.

## Known risks and limitations

Wire-level normalization may break clients even when Rust types remain
compatible. Bounded labels require application-controlled operation categories;
raw resource paths are not safe substitutes. Shared correlation is not shared
identity or shared authorization.

## Architectural rationale

Purpose-specific adapters reuse the existing feature boundaries and can be
adopted independently. Compatibility fixtures supply stronger evidence than
assuming similarly named consumer helpers have identical behaviour.
