# Add unit and integration tests for the shared SSE module

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DRAFT

## Purpose / big picture

Roadmap task 1.2.1 is the proof stage for the shared Server-Sent Events (SSE)
wire helpers defined by [Architecture Decision Record (ADR) 001][adr-001].
Tasks 1.1.1, 1.1.2, and 1.1.3 already landed the transport-focused helper
surface in `src/sse/`. This task does not broaden that surface. It proves that
the existing public API behaves exactly as the contract says it should.

After this change, the repository should give maintainers and downstream users
two forms of evidence:

1. Fine-grained unit coverage, written with `rstest`, for each helper's happy
   path, unhappy path, and edge cases.
2. A small integration-level contract suite, written with `rstest-bdd` where
   the scenario structure genuinely helps, that exercises the crate's exported
   SSE API the way a downstream consumer would use it.

Success is observable when all roadmap-mandated behaviours are proven:
identifier validation, `Last-Event-ID` parsing, frame formatting, cache
headers, heartbeat output, and `stream_reset` output. It is also observable
when `make check-fmt`, `make lint`, and `make test` pass with the new coverage,
and when `docs/roadmap.md` marks task 1.2.1 done only after the implementation
is complete.

This plan must be approved before any implementation begins.

[adr-001]: ../adr-001-shared-sse-wire-contract-for-wildside-and-corbusier.md

## Constraints

- Keep the SSE module wire-only. Do not pull event-store, routing,
  authorization, responder lifecycle, or scheduler logic into this task.
- Treat the already-landed SSE module as the system under test. The expected
  implementation targets are:
  - `src/sse/event_id.rs`
  - `src/sse/replay_cursor.rs`
  - `src/sse/frame.rs`
  - `src/sse/cache_control.rs`
  - `src/sse/heartbeat.rs`
  - `src/sse/stream_reset.rs`
- Preserve the current module boundary: detailed helper-specific assertions
  belong in unit tests near the module they exercise; integration tests should
  prove the exported contract rather than duplicating every unit assertion.
- Use `rstest` for unit coverage and parameterized edge cases. Use
  `rstest-bdd` only when a downstream-style scenario is clearer than a direct
  Rust assertion.
- Cover the roadmap bullet list exactly:
  - identifier validation;
  - `Last-Event-ID` parsing;
  - frame formatting;
  - cache headers;
  - heartbeat output;
  - `stream_reset` output.
- Retain the UTF-8 replay-cursor regression coverage. `Last-Event-ID` parsing
  must continue to accept valid UTF-8 that `HeaderValue::to_str()` would have
  rejected.
- Retain the existing HTTP-header reality: CR, LF, and NULL cannot be created
  as valid `HeaderValue`s, so those bytes must be tested at the identifier
  validation layer rather than by manufacturing impossible headers.
- Any decision that sharpens or changes the normative wire contract must be
  recorded in ADR 001 before or alongside the test change.
- `docs/users-guide.md` must be updated only if the test audit reveals a user-
  visible behaviour clarification or example gap. `docs/developers-guide.md`
  must be updated with any internal testing practice or layout change that the
  next maintainer needs to know.
- `docs/roadmap.md` must not mark task 1.2.1 done until the implementation,
  documentation, and all quality gates are complete.
- Follow the repository documentation and language conventions, including
  en-GB-oxendict spelling, wrapped Markdown, module-level `//!` comments, and
  documented public interfaces.

## Tolerances (exception triggers)

- Scope: if closing task 1.2.1 requires broad production-code changes beyond
  narrow testability fixes or documentation clarifications, stop and escalate.
  This task should be mostly tests plus small supporting adjustments.
- Test layout: if the new coverage cannot fit cleanly into the existing module
  tests plus one integration test file and one feature file, stop and explain
  why a broader layout is needed.
- Public API: if proving the contract requires changing the exported SSE API,
  stop and escalate before implementation. This task is to prove the contract,
  not redesign it.
- Specification ambiguity: if ADR 001, the existing docs, and the current code
  disagree on an observable SSE behaviour, stop, record the conflict, and
  resolve it in ADR 001 before changing the tests.
- Iterations: if the same failing gate or test issue survives three focused fix
  attempts, stop and document the blocker instead of improvising around it.
- Dependencies: no new crates are expected. If a new dependency appears
  necessary, stop and escalate.

## Risks

- Risk: duplicating existing module tests in a new integration suite would
  increase maintenance cost without adding confidence. Severity: medium.
  Likelihood: high. Mitigation: begin with a coverage audit, then add unit
  tests only where gaps remain and keep the integration suite focused on
  end-to-end contract composition through the public re-exports.

- Risk: a test may accidentally codify implementation detail rather than the
  normative wire contract. Severity: medium. Likelihood: medium. Mitigation:
  use ADR 001 and the existing user/developer guides as the source of truth,
  and update ADR 001 whenever a test introduces a newly explicit contract
  decision.

- Risk: behavioural tests could add ceremony without increasing clarity.
  Severity: low. Likelihood: medium. Mitigation: use `rstest-bdd` only for
  downstream-style scenarios that combine multiple helpers; keep the detailed
  edge-case matrix in `rstest` unit tests.

- Risk: a test audit may reveal that one or more existing helper tests are
  missing a roadmap-required unhappy path. Severity: medium. Likelihood:
  medium. Mitigation: treat the audit as Stage A, record gaps explicitly, then
  add focused tests instead of rewriting whole modules.

- Risk: `make lint` can appear stalled because Whitaker builds its Dylint
  driver on first use. Severity: low. Likelihood: medium. Mitigation: keep the
  logged gate output, wait for the build to finish, and do not assume the lint
  job is hung if the process remains active.

## Progress

- [x] 2026-04-21: Read `docs/roadmap.md`, ADR 001, the prior SSE execplans,
  and the current `src/sse/` implementation.
- [x] 2026-04-21: Review the referenced testing guidance:
  `docs/rust-testing-with-rstest-fixtures.md`,
  `docs/reliable-testing-in-rust-via-dependency-injection.md`,
  `docs/rust-doctest-dry-guide.md`, and
  `docs/complexity-antipatterns-and-refactoring-strategies.md`.
- [x] 2026-04-21: Draft this execution plan.
- [ ] Await user approval of this plan.
- [ ] Audit existing unit coverage against roadmap task 1.2.1 and ADR 001.
- [ ] Add or tighten module-level `rstest` coverage for any missing SSE edge
  cases.
- [ ] Add a downstream-facing integration contract suite for the shared SSE
  module.
- [ ] Update ADR 001, `docs/users-guide.md`, and `docs/developers-guide.md` as
  required by the final implementation findings.
- [ ] Run `make check-fmt`, `make lint`, `make test`, `make fmt`,
  `make markdownlint`, and `make nixie` with `tee` logs.
- [ ] Mark roadmap task 1.2.1 done after the feature is complete.

## Surprises & Discoveries

- 2026-04-21: The production SSE helpers and substantial unit coverage already
  exist from tasks 1.1.1 through 1.1.3. Task 1.2.1 is therefore primarily a
  contract-proof and coverage-audit task, not a greenfield testing task.
- 2026-04-21: The repo already carries `rstest-bdd` infrastructure under
  `tests/` and `tests/features/`, so the SSE integration suite can follow an
  established pattern instead of inventing a new one.
- 2026-04-21: The SSE regression notes from earlier work materially constrain
  the test design:
  - invalid CR/LF/NULL bytes belong in identifier validation tests, not header
    construction tests;
  - UTF-8 replay cursor coverage must remain explicit.

## Decision Log

- 2026-04-21: This plan recommends starting with a coverage audit instead of
  blindly adding a second copy of existing tests. Rationale: the current module
  tests already prove many behaviours, and task 1.2.1 should close the
  remaining gaps rather than inflate the suite with redundant assertions.

- 2026-04-21: This plan recommends one focused integration suite that consumes
  the crate's public SSE re-exports from `tests/`, backed by one `.feature`
  file if the scenario structure adds clarity. Rationale: integration coverage
  should prove the published contract as a downstream crate sees it, not reach
  into module-private helpers.

- 2026-04-21: This plan treats documentation updates as conditional rather than
  automatic. Rationale: task 1.2.1 should not manufacture user-facing behaviour
  changes; it should only update ADR 001 and the guides when the testing work
  makes an existing contract statement clearer or more precise.

## Outcomes & Retrospective

Pending implementation. When task 1.2.1 is complete, replace this section with
the final test layout, the exact behaviours proven, the gate results, and any
lessons that should inform later SSE adoption work.

## Context and orientation

### Current SSE surface

The current crate exports the following SSE contract from `src/lib.rs` and
`src/sse/mod.rs`:

```plaintext
EventId
EventIdValidationError
ReplayCursor
ReplayCursorError
LAST_EVENT_ID_HEADER
extract_replay_cursor
map_replay_cursor_error
render_event_frame
render_comment_frame
SseFrameError
EVENT_STREAM_CACHE_CONTROL
apply_event_stream_cache_control
DEFAULT_HEARTBEAT_INTERVAL
HeartbeatPolicy
HeartbeatPolicyError
render_heartbeat_frame
STREAM_RESET_EVENT_NAME
STREAM_RESET_REPLAY_UNAVAILABLE_PAYLOAD
render_stream_reset_frame
```

The detailed helper tests already live beside the implementations in
`src/sse/*.rs`. Task 1.2.1 should extend that pattern rather than moving tests
out of the modules.

### Existing test conventions to reuse

The repository already uses two relevant patterns:

1. Module-local `rstest` unit tests for precise validation and regression
   coverage.
2. Behavioural tests in `tests/` plus plain-text feature files in
   `tests/features/` for downstream contract scenarios.

The existing BDD examples are:

- `tests/pagination_bdd.rs`
- `tests/openapi_schemas_bdd.rs`
- `tests/features/pagination.feature`
- `tests/features/openapi_schemas.feature`

Task 1.2.1 should copy these conventions only where they make the SSE contract
clearer.

### Recommended target layout

Unless the coverage audit proves a simpler option, implementation should stay
within this layout:

```plaintext
src/sse/event_id.rs         # extend unit coverage if identifier gaps exist
src/sse/replay_cursor.rs    # extend unit coverage if header-parsing gaps exist
src/sse/frame.rs            # extend unit coverage if framing gaps exist
src/sse/cache_control.rs    # extend unit coverage if header-policy gaps exist
src/sse/heartbeat.rs        # extend unit coverage if heartbeat gaps exist
src/sse/stream_reset.rs     # extend unit coverage if stream_reset gaps exist
tests/sse_wire_contract_bdd.rs
tests/features/sse_wire_contract.feature
```

This split keeps edge-case detail near the implementation while giving the
crate one downstream-facing proof that the exported helpers compose into the
approved SSE wire contract.

## References and supporting skills

### Governing documents

- [Roadmap](../roadmap.md) records this work as task 1.2.1.
- [ADR 001][adr-001] is the normative SSE contract.
- [Users guide](../users-guide.md) records the public SSE API expectations.
- [Developers guide](../developers-guide.md) records module boundaries, test
  posture, and SSE maintenance guidance.
- [Task 1.1.1 execplan](1-1-1-implement-sse-identifier-and-replay-cursor-helpers.md)
  explains the identifier and replay-cursor constraints.
- [Task 1.1.2 execplan](1-1-2-sse-frame-and-cache-header-helpers.md) explains
  framing and cache-policy constraints.
- [Task 1.1.3 execplan](1-1-3-shared-heartbeat-and-stream-reset-helpers.md)
  explains heartbeat and `stream_reset` constraints.

### Testing and maintenance references

- [Rust testing with rstest fixtures](../rust-testing-with-rstest-fixtures.md)
  for fixtures and parameterized test structure.
- [Reliable testing in Rust via dependency
  injection](../reliable-testing-in-rust-via-dependency-injection.md) for
  deterministic testing choices and avoiding global-state mutation.
- [Rust doctest DRY guide](../rust-doctest-dry-guide.md) for deciding whether
  any public SSE examples should be tightened while avoiding duplication.
- [Complexity antipatterns and refactoring
  strategies](../complexity-antipatterns-and-refactoring-strategies.md) for
  resisting bloated test helpers and keeping the new test code readable.

### Relevant skills

- `execplans`: keep this document current during implementation, and do not
  start coding until the user approves it.
- `leta`: use first when navigating SSE symbols, existing tests, and re-exports
  before editing code.
- `rust-router`: use to route implementation questions to narrower Rust skills
  if the test work uncovers design or API issues.
- `code-review`: use after implementation to review the final diff for missing
  coverage, accidental regressions, or brittle tests.

## Build and validation commands

All formal gates must use `tee` and `set -o pipefail` so the log survives long
or truncated output. Run them sequentially.

For the implementation turn:

```bash
set -o pipefail && make check-fmt 2>&1 | tee /tmp/check-fmt-sse-tests.out
set -o pipefail && make lint 2>&1 | tee /tmp/lint-sse-tests.out
set -o pipefail && make test 2>&1 | tee /tmp/test-sse-tests.out
set -o pipefail && make fmt 2>&1 | tee /tmp/fmt-sse-tests.out
set -o pipefail && make markdownlint 2>&1 | tee /tmp/markdownlint-sse-tests.out
set -o pipefail && make nixie 2>&1 | tee /tmp/nixie-sse-tests.out
```

For this plan-only documentation turn:

```bash
set -o pipefail && make fmt 2>&1 | tee /tmp/fmt-sse-tests-plan.out
set -o pipefail && make markdownlint 2>&1 | tee /tmp/markdownlint-sse-tests-plan.out
set -o pipefail && make nixie 2>&1 | tee /tmp/nixie-sse-tests-plan.out
```

## Plan of work

The implementation should proceed in four stages. Do not begin Stage B until
the user approves this plan.

### Stage A: audit the existing SSE coverage against the roadmap

Start by comparing the current tests in `src/sse/*.rs` against the exact
behaviours named in roadmap task 1.2.1 and ADR 001. Produce a short coverage
matrix in the working notes or update this plan's `Decision Log` if anything
surprising appears.

Questions the audit must answer:

1. Which roadmap-required behaviours are already covered well?
2. Which behaviours are covered only indirectly and need a clearer direct test?
3. Which gaps belong in unit tests versus the integration contract suite?
4. Does any existing test codify behaviour that ADR 001 does not yet state
   clearly?

The audit should explicitly check:

- forbidden-character and empty-value identifier validation;
- UTF-8, missing, empty, duplicate, and malformed `Last-Event-ID` cases;
- deterministic frame ordering and newline normalization;
- canonical cache-control application and repeatability;
- default heartbeat policy, explicit override, and canonical heartbeat frame;
- fixed `stream_reset` event name, payload, and rendered output.

Validation for Stage A:

- the gap list is explicit;
- no existing coverage is deleted without replacement;
- any contract ambiguity is recorded before new assertions are added.

### Stage B: close the unit-test gaps with `rstest`

Add or refine unit tests in the individual SSE modules based on the Stage A
audit. Resist the urge to move or rewrite working tests simply for cosmetic
consistency.

Expected module-level responsibilities:

1. `src/sse/event_id.rs`
   Verify happy paths, empty identifiers, forbidden CR/LF/NULL characters,
   whitespace preservation, and any constructor or conversion edge cases that
   are still uncovered.
2. `src/sse/replay_cursor.rs`
   Verify crate-root header parsing rules, especially UTF-8 acceptance, missing
   and empty headers, duplicate headers, and invalid-header mapping.
3. `src/sse/frame.rs`
   Verify `id:`, `event:`, `data:`, and comment rendering, including logical
   newline handling, empty event-name rejection, Unicode-safe output, and any
   invalid NULL payload cases.
4. `src/sse/cache_control.rs`
   Verify the canonical `Cache-Control` value, deterministic replacement of any
   prior value, and preservation of unrelated headers.
5. `src/sse/heartbeat.rs`
   Verify the 20-second default, explicit override acceptance, zero-interval
   rejection, and canonical heartbeat frame output.
6. `src/sse/stream_reset.rs`
   Verify the fixed event name, fixed payload, rendered frame, and
   repeatability.

Validation for Stage B:

- every roadmap bullet has direct unit coverage or an intentional integration
  counterpart;
- new module tests use `rstest` parameterization where it improves readability;
- no helper file becomes bloated or harder to follow because of the added
  tests.

### Stage C: add the downstream-facing integration contract suite

Create `tests/sse_wire_contract_bdd.rs` and
`tests/features/sse_wire_contract.feature` if the Stage A audit confirms that a
scenario-oriented suite adds clarity. The integration suite should exercise the
crate's public exports from the perspective of a downstream consumer and should
compose helpers rather than reaching into internals.

Recommended scenarios:

1. A reconnect request with a valid `Last-Event-ID` produces a replay cursor
   that preserves the supplied identifier.
2. A live stream response uses the canonical cache policy and canonical
   heartbeat frame.
3. A normal event frame and the fixed `stream_reset` control frame render the
   approved wire format through the public API.

If Stage A shows that `rstest-bdd` would add more ceremony than value, record
that decision in `Decision Log` and replace this stage with one plain
integration test file under `tests/` using direct Rust assertions. Do not force
BDD if it does not improve the proof.

Validation for Stage C:

- the new integration suite imports from the crate root, not module-private
  paths;
- the scenarios prove the published wire contract rather than duplicate every
  unit-test permutation;
- the chosen style (`rstest-bdd` or plain integration tests) is justified in
  the `Decision Log`.

### Stage D: documentation review, gates, and closure

Once the tests are green, perform the documentation review and close the task.

Required closure work:

1. Update ADR 001 if the final tests make any contract detail newly explicit.
2. Update `docs/users-guide.md` only if the audit or final tests expose a
   public-behaviour clarification, missing example, or wording mismatch.
3. Update `docs/developers-guide.md` with any new internal testing convention,
   file layout, or maintenance note that future contributors need.
4. Run the full gate set with `tee` logs:
   `make check-fmt`, `make lint`, `make test`, `make fmt`, `make markdownlint`,
   and `make nixie`.
5. Mark task 1.2.1 as done in `docs/roadmap.md` only after all gates pass.
6. Update this plan's `Progress`, `Decision Log`, `Surprises & Discoveries`,
   and `Outcomes & Retrospective` sections with the actual results.

Validation for Stage D:

- all applicable code and documentation gates pass;
- roadmap task 1.2.1 is the only roadmap item closed by this feature;
- the final diff demonstrates contract proof rather than accidental feature
  expansion.

## Acceptance criteria

The task is complete only when all of the following are true:

1. The shared SSE module has unit and integration coverage that collectively
   proves every roadmap bullet in task 1.2.1.
2. The implementation keeps the crate wire-only and does not expand the SSE
   feature set beyond testing or small supporting clarifications.
3. Any contract decision sharpened by the tests is written into ADR 001.
4. Any required guide updates are landed in `docs/users-guide.md` and
   `docs/developers-guide.md`.
5. `make check-fmt`, `make lint`, `make test`, `make fmt`,
   `make markdownlint`, and `make nixie` all pass.
6. `docs/roadmap.md` marks task 1.2.1 done, and this execplan is updated to
   reflect the completed work.
