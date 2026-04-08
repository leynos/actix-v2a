# Implement SSE frame and cache-header helpers

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DRAFT

## Purpose / big picture

`actix-v2a` already exposes the first slice of the shared Server-Sent Events
(SSE) contract from [ADR 001][adr-001]: validated `EventId` values and
`Last-Event-ID` replay cursor parsing. Roadmap task 1.1.2 is the next delivery
unit in that contract. It adds the wire helpers that downstream applications
need to emit browser-compatible SSE frames and to mark live event-stream
responses as non-reusable by intermediaries.

After this change, downstream services should be able to do three things
without reimplementing wire details:

1. Render deterministic SSE event frames that can include `id:`, `event:`,
   and `data:` fields.
2. Render comment frames suitable for heartbeat traffic without inventing fake
   domain events.
3. Apply a canonical cache-control policy for live event streams so browsers
   and shared intermediaries do not reuse stale stream responses.

This work remains transport-focused. It must not pull event-store retention,
authorization policy, route design, or an Actix convenience responder into the
crate. The goal is a small shared helper surface that task 1.1.3 can build on
for heartbeat policy and `stream_reset`, while Wildside and Corbusier continue
to own their stream lifecycle and orchestration code.

Success is observable when:

- `src/sse/` exposes approved frame and cache helpers alongside the existing
  identifier and replay cursor types.
- New unit tests prove happy paths, unhappy paths, and framing edge cases using
  `rstest`.
- Behavioural tests use `rstest-bdd` only if a scenario-level assertion adds
  clarity beyond straightforward unit coverage.
- `docs/users-guide.md`, `docs/developers-guide.md`, and ADR 001 describe the
  approved helper behaviour.
- `docs/roadmap.md` marks task 1.1.2 done only after all quality gates pass.

[adr-001]: ../adr-001-shared-sse-wire-contract-for-wildside-and-corbusier.md

## Constraints

- Build on the existing `src/sse/` module and reuse `EventId` rather than
  duplicating identifier validation logic.
- Keep the implementation wire-only. Do not add:
  - event-store interfaces;
  - replay retention policy;
  - route or authorization logic;
  - an Actix responder or stream lifecycle abstraction.
- Frame helpers must emit valid SSE wire text for:
  - event frames containing `id:`, `event:`, and `data:` fields as needed;
  - comment frames for heartbeat traffic.
- Cache helpers must disable intermediary reuse of live event streams without
  reaching into proxy-vendor-specific buffering controls unless that behaviour
  is separately approved.
- The helper surface must remain usable from `actix-web` handlers with the
  dependencies already present in `Cargo.toml`.
- Public Rust interfaces require Rustdoc comments with examples, and every
  module file requires a module-level `//!` comment.
- New code must satisfy the repository lint posture, including deny-level
  Clippy rules such as `expect_used`, `unwrap_used`, `cognitive_complexity`,
  and `missing_const_for_fn`.
- File size must remain below the repository guidance of 400 lines per file.
- Tests must use `rstest` fixtures and parameterized cases where they improve
  clarity. `rstest-bdd` should be used only where scenario structure adds real
  value.
- Any design decision that refines ADR 001 must be recorded in the ADR, not
  only in code comments or tests.

## Tolerances (exception triggers)

- Scope: if task 1.1.2 requires more than four new source files or roughly 450
  net lines of Rust, stop and re-scope before implementation.
- Dependencies: no new external crates are expected. If one appears
  necessary, stop and escalate.
- Public API shape: if the smallest coherent helper surface cannot be expressed
  without introducing a responder, generic stream abstraction, or application
  callback hooks, stop and escalate.
- Specification ambiguity: if newline handling, empty event-name semantics, or
  the exact cache header set materially affect downstream behaviour and ADR 001
  does not already settle the choice, stop and record the proposed decision in
  the ADR before code lands.
- Iterations: if the same failing test or lint issue persists after three
  focused fix attempts, stop and document the blocker.

## Risks

- Risk: over-designing a small framing task into a builder or responder API
  that leaks lifecycle concerns into `actix-v2a`. Severity: medium. Likelihood:
  medium. Mitigation: keep the public surface rendering-focused and reject
  convenience responder work in this milestone.

- Risk: incorrect handling of multi-line `data:` payloads or trailing newline
  cases could produce subtly invalid SSE output. Severity: high. Likelihood:
  medium. Mitigation: add explicit unit coverage for empty payloads, embedded
  blank lines, Unicode data, and trailing newline cases, and document any
  chosen normalisation rule in ADR 001.

- Risk: cache-control helpers may be either too weak (shared caches still
  reuse streams) or broader than the ADR intends. Severity: medium. Likelihood:
  medium. Mitigation: pick one canonical anti-reuse policy, prove it in tests,
  and document why vendor-specific headers remain out of scope.

- Risk: behavioural tests may add ceremony without increasing confidence
  because the task is mostly deterministic string and header formatting.
  Severity: low. Likelihood: high. Mitigation: prefer `rstest` unit tests and
  record explicitly if `rstest-bdd` is deemed unnecessary for this slice.

## Progress

- [x] Read `docs/roadmap.md`, ADR 001, the existing SSE module, and the prior
  task 1.1.1 execplan.
- [x] Draft this execution plan.
- [ ] Receive user approval for the plan.
- [ ] Implement the SSE frame helper module.
- [ ] Implement the SSE cache-header helper module.
- [ ] Add unit tests and any justified behavioural tests.
- [ ] Update ADR 001, `docs/users-guide.md`, and `docs/developers-guide.md`.
- [ ] Mark roadmap task 1.1.2 done after the implementation passes all gates.
- [ ] Run `make check-fmt`, `make lint`, `make test`, `make fmt`,
  `make markdownlint`, and `make nixie` with `tee` logs.

## Context and orientation

### Current SSE module state

The repository already contains:

```text
src/sse/
  mod.rs            # module docs and re-exports
  event_id.rs       # EventId validation
  replay_cursor.rs  # Last-Event-ID parsing and mapping
```

`src/lib.rs` re-exports `EventId`, `ReplayCursor`, `LAST_EVENT_ID_HEADER`,
`ReplayCursorError`, `extract_replay_cursor`, and `map_replay_cursor_error`.
Task 1.1.2 should extend this surface rather than introducing a parallel SSE
module elsewhere.

### Relevant existing patterns

- `EventId` already provides the validated string needed for any `id:` field.
  The frame helpers should accept that type where an identifier is present
  rather than taking an unvalidated `&str`.
- The idempotency module shows the repository's usual pattern for transport
  helpers: typed validation plus small Actix-friendly helpers, not full
  responders.
- Documentation and tests added for task 1.1.1 already establish the SSE
  section in `docs/users-guide.md` and `docs/developers-guide.md`; task 1.1.2
  should extend those sections instead of creating parallel documents.

### Recommended target layout

Unless a sharper split appears during implementation, add:

```text
src/sse/
  frame.rs          # SSE event/comment frame rendering helpers
  cache_control.rs  # event-stream cache header helpers
```

Then update:

- `src/sse/mod.rs` for module declarations and re-exports.
- `src/lib.rs` for crate-root re-exports.

This keeps frame rendering and header policy separate, keeps files under the
400-line limit, and leaves room for task 1.1.3 to add heartbeat policy and
`stream_reset` without overcrowding one file.

## Build and validation commands

Implementation work must capture every gate with `tee` and `set -o pipefail`.
Run the gates sequentially, never in parallel.

```bash
set -o pipefail && make check-fmt 2>&1 | tee /tmp/check-fmt-sse-frame-helpers.out
set -o pipefail && make lint 2>&1 | tee /tmp/lint-sse-frame-helpers.out
set -o pipefail && make test 2>&1 | tee /tmp/test-sse-frame-helpers.out
set -o pipefail && make fmt 2>&1 | tee /tmp/fmt-sse-frame-helpers.out
set -o pipefail && make markdownlint 2>&1 | tee /tmp/markdownlint-sse-frame-helpers.out
set -o pipefail && make nixie 2>&1 | tee /tmp/nixie-sse-frame-helpers.out
```

For this plan-only change, run only the documentation gates after editing the
Markdown files. Once implementation begins, rerun the full Rust and
documentation gate set before commit.

## Plan of work

The implementation should proceed in four stages. Each stage ends with a
validation step, and later stages should not begin until the current stage is
green.

### Stage A: lock the helper surface and ADR refinements

Before writing code, confirm the smallest useful public API for this task. The
recommended direction is:

- one helper for rendering event frames with optional `id:` and `event:`
  fields and required `data:` output;
- one helper for rendering comment frames used by heartbeat traffic;
- one helper for applying the canonical no-reuse cache policy to a header map
  or equivalent typed header surface.

This stage must also settle any framing details that ADR 001 leaves implicit:

- how multi-line `data:` payloads are split into repeated `data:` lines;
- whether bare `\r`, `\n`, and `\r\n` in data are normalised as logical line
  breaks or rejected;
- whether an explicitly empty event name is permitted, or omission is the only
  supported way to express the default browser `message` event;
- the exact cache-control directive set for live streams.

If any of these choices changes or sharpens the normative contract, update ADR
001 before implementation proceeds.

Validation for Stage A:

- the user approves this plan;
- ADR 001 is ready to accept any needed clarifications;
- no responder or lifecycle abstraction has leaked into the proposed API.

### Stage B: implement frame rendering helpers

Create `src/sse/frame.rs` with a rendering-focused API. Keep the helpers small
and composable rather than introducing a responder or runtime abstraction.

Implementation expectations:

- Reuse `EventId` for any `id:` output.
- Emit complete SSE frames terminated by a blank line.
- Produce deterministic field ordering so tests and downstream output are
  stable.
- Support multi-line `data:` payloads by rendering one `data:` line per
  logical line of payload text.
- Provide a comment-frame helper that can represent heartbeat traffic without
  using fake event names or payloads.
- Validate any untyped textual inputs only as far as needed to preserve wire
  integrity, and record the exact rule in ADR 001 if it becomes part of the
  public contract.

Unit test coverage in this stage should include at least:

- event frame with `id`, `event`, and single-line `data`;
- event frame with omitted optional fields;
- multi-line `data:` payload rendering;
- empty or blank-line payload behaviour;
- Unicode payload and event name handling where permitted;
- comment heartbeat frame rendering;
- rejection paths for any invalid wire-breaking input chosen by the approved
  API.

`rstest-bdd` is probably unnecessary here because the behaviour is pure
formatting. If that remains true during implementation, record that decision in
the execplan and keep the tests as `rstest` unit coverage.

Validation for Stage B:

- `make check-fmt`
- `make lint`
- targeted review of rendered output in tests

### Stage C: implement cache-header helpers

Create `src/sse/cache_control.rs` with the canonical event-stream anti-cache
policy. Keep the helper Actix-friendly, but do not tie it to an Actix responder
type.

Recommended direction:

- expose either a small function that mutates a `HeaderMap` or a typed helper
  that yields the approved header/value pairs;
- prefer standard cache headers only;
- leave proxy-specific buffering or transport tuning headers out of scope for
  this task.

Unit test coverage in this stage should include at least:

- helper applies the expected `Cache-Control` value;
- helper replaces or deterministically sets existing cache-control state;
- helper does not modify unrelated headers;
- repeated application is idempotent or at least deterministic.

If a minimal scenario-level test using Actix response headers materially
improves clarity, add one `rstest-bdd` behavioural test. If not, document why
unit tests are sufficient.

Validation for Stage C:

- `make check-fmt`
- `make lint`
- `make test`

### Stage D: documentation, roadmap, and final evidence capture

After code and tests are green, update the documentation set:

1. `docs/adr-001-shared-sse-wire-contract-for-wildside-and-corbusier.md`
   records any design decisions taken during implementation, especially newline
   handling, event-name treatment, and the chosen cache policy.
2. `docs/users-guide.md` documents the new public helper surface with concise
   examples for rendering SSE frames and applying live-stream cache headers. It
   must also state explicitly that this crate still does not provide a
   convenience responder.
3. `docs/developers-guide.md` documents the internal module split, the chosen
   wire-formatting rules, testing rationale, and the boundary that keeps
   responder/lifecycle concerns out of scope.
4. `docs/roadmap.md` marks task 1.1.2 done only after all implementation and
   documentation gates pass.

Final validation for Stage D:

- `make fmt`
- `make markdownlint`
- `make nixie`
- `make check-fmt`
- `make lint`
- `make test`

## Approval gates

Implementation must not begin until all of the following are true:

1. The user approves this plan.
2. The approved surface remains wire-only and excludes any convenience
   responder.
3. Any ADR clarification needed for newline handling or cache policy has been
   accepted before code lands.

## Validation and acceptance

Task 1.1.2 is done when all of the following are true:

- `src/sse/` exports shared frame and cache-header helpers that remain
  transport-focused.
- Unit tests cover happy paths, unhappy paths, and framing edge cases with
  `rstest`.
- Any behavioural tests added with `rstest-bdd` are justified by scenario
  clarity; otherwise the execplan explicitly records why they were not needed.
- `make check-fmt`, `make lint`, `make test`, `make fmt`,
  `make markdownlint`, and `make nixie` all pass.
- ADR 001, `docs/users-guide.md`, and `docs/developers-guide.md` reflect the
  approved helper behaviour.
- `docs/roadmap.md` marks 1.1.2 complete.

## Idempotence and recovery

The implementation should be additive and safe to rerun. If a stage fails,
repair the issue and rerun that stage's validation commands. No rollback should
be necessary unless the attempted API expands beyond the approved wire-helper
scope.

## Surprises & discoveries

- 2026-04-08: the current repository already contains the completed 1.1.1 SSE
  work (`EventId` and `ReplayCursor`), so task 1.1.2 can plan directly against
  the live `src/sse/` module rather than a hypothetical scaffold.
- 2026-04-08: no `execplans` skill was available in this session, so this plan
  was drafted using the repository's existing execplan pattern and local
  documentation guidance.

## Decision log

- 2026-04-08: recommended a split between `frame.rs` and
  `cache_control.rs` so the framing rules and cache policy stay independently
  testable and under the repository's 400-line file guidance.
- 2026-04-08: recommended keeping task 1.1.2 strictly below the responder
  boundary so task 1.1.3 can build heartbeat policy and `stream_reset` on top
  of stable frame primitives rather than forcing a premature lifecycle API.

## Outcomes & retrospective

Not started. Populate this section after implementation and gate completion.
