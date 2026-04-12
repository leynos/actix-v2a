# Implement shared heartbeat and stream_reset helpers

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETED

## Purpose / big picture

`actix-v2a` already contains the first two slices of the shared Server-Sent
Events (SSE) contract from [Architecture Decision Record (ADR) 001][adr-001]:
validated event identifiers and replay cursors, plus shared frame-rendering and
live-stream cache-control helpers. Roadmap task 1.1.3 is the next delivery unit
in that contract. It adds the transport-level helpers that downstream
applications need to keep idle SSE connections warm and to signal that replay
can no longer resume from a supplied cursor.

After this change, downstream services should be able to do three things
without re-implementing small but important pieces of wire policy:

1. Use one shared default heartbeat interval of 20 seconds.
2. Override that interval explicitly when deployment conditions require a
   different heartbeat cadence.
3. Emit one standard `stream_reset` event whose payload is exactly
   `{"reason":"replay_unavailable"}`.

This work must remain wire-only. It must not pull timer scheduling, event-store
retention policy, route design, responder lifecycle, or authorization logic
into the crate. The goal is a narrow helper surface that composes with the
existing frame helpers from task 1.1.2 while leaving applications in full
control of when a heartbeat is sent and when replay is declared unavailable.

Success is observable when:

- `src/sse/` exports the approved heartbeat policy helper and a standard
  `stream_reset` helper alongside the existing frame helpers.
- The default heartbeat interval is 20 seconds and an explicit override path is
  available through the public API.
- The `stream_reset` helper emits the exact ADR-defined event name and payload
  without introducing generic application-event helpers.
- New unit tests cover happy paths, unhappy paths, and relevant edge cases
  using `rstest`.
- Behavioural tests use `rstest-bdd` only if a scenario-level assertion adds
  clarity beyond straightforward unit coverage.
- ADR 001, `docs/users-guide.md`, and `docs/developers-guide.md` describe the
  approved helper behaviour.
- `docs/roadmap.md` marks task 1.1.3 done only after all quality gates pass.

[adr-001]: ../adr-001-shared-sse-wire-contract-for-wildside-and-corbusier.md

## Constraints

- Build on the existing `src/sse/` module and reuse the landed helpers from
  tasks 1.1.1 and 1.1.2 instead of re-implementing framing rules.
- Keep the implementation wire-only. Do not add:
  - timer or scheduler ownership;
  - async stream abstractions;
  - event-store interfaces or retention policy;
  - route or authorization logic;
  - an Actix responder or connection-lifecycle abstraction.
- Provide the ADR default heartbeat interval of 20 seconds and an explicit
  override path in the public API.
- Emit the standard `stream_reset` event with the exact event name
  `stream_reset` and the exact payload `{"reason":"replay_unavailable"}`.
- Keep application event names and payload schemata out of scope.
- Prefer a small, typed helper surface over raw magic strings in downstream
  code.
- Public Rust interfaces require Rustdoc comments with examples, and every
  module file requires a module-level `//!` comment.
- New code must satisfy the repository lint posture, including deny-level
  Clippy rules such as `expect_used`, `unwrap_used`, `missing_const_for_fn`,
  and `cognitive_complexity`.
- File size must remain below the repository guidance of 400 lines per file.
- Tests must use `rstest` fixtures and parameterized cases where they improve
  clarity. `rstest-bdd` should be used only where scenario structure adds real
  value.
- Any design decision that sharpens ADR 001 must be recorded in the ADR, not
  only in code comments or tests.

## Tolerances (exception triggers)

- Scope: if task 1.1.3 requires more than four new source files or roughly 350
  net lines of Rust, stop and re-scope before implementation.
- Dependencies: no new external crates are expected. If one appears
  necessary, stop and escalate.
- Public API shape: if the smallest coherent helper surface cannot be
  expressed without introducing a scheduler, responder, or application callback
  hooks, stop and escalate.
- Specification ambiguity: if the zero-duration override case, heartbeat frame
  shape, or `stream_reset` helper signature materially affects downstream
  behaviour and ADR 001 does not already settle the choice, stop and record the
  proposed decision in the ADR before code lands.
- Iterations: if the same failing test or lint issue persists after three
  focused fix attempts, stop and document the blocker.

## Risks

- Risk: conflating a heartbeat policy helper with runtime scheduling could push
  `actix-v2a` past the wire-helper boundary. Severity: high. Likelihood:
  medium. Mitigation: keep the public surface limited to interval data and
  rendering helpers only; do not add background-task or timer orchestration.

- Risk: `stream_reset` could sprawl into a generic application-event builder,
  which would violate the ADR's scope boundary. Severity: medium. Likelihood:
  medium. Mitigation: expose only the fixed `stream_reset` event name and
  payload defined by ADR 001.

- Risk: the explicit override path may leave zero or otherwise degenerate
  intervals underspecified. Severity: medium. Likelihood: medium. Mitigation:
  settle the constructor semantics up front, document them in ADR 001 if they
  become normative, and cover them with tests.

- Risk: `rstest-bdd` may add ceremony without increasing confidence because the
  feature is deterministic rendering plus interval validation. Severity: low.
  Likelihood: high. Mitigation: prefer `rstest` unit tests and record
  explicitly if behavioural coverage is unnecessary for this slice.

## Progress

- [x] Read `docs/roadmap.md`, ADR 001, the current SSE module, and the prior
  execplans for tasks 1.1.1 and 1.1.2.
- [x] Review the referenced testing and documentation guidance.
- [x] Draft this execution plan.
- [x] Receive user approval for the plan.
- [x] Implement the heartbeat policy helper.
- [x] Implement the `stream_reset` helper.
- [x] Add unit tests and any justified behavioural tests.
- [x] Update ADR 001, `docs/users-guide.md`, and `docs/developers-guide.md`.
- [x] Mark roadmap task 1.1.3 done after the implementation passes all gates.
- [x] Run `make check-fmt`, `make lint`, `make test`, `make fmt`,
  `make markdownlint`, and `make nixie` with `tee` logs for the implementation
  change.

## Context and orientation

### Current SSE module state

The repository already contains:

```plaintext
src/sse/
  mod.rs            # module docs and re-exports
  event_id.rs       # EventId validation
  replay_cursor.rs  # Last-Event-ID parsing and mapping
  frame.rs          # event and comment frame rendering
  cache_control.rs  # live-stream cache policy helpers
```

`src/lib.rs` already re-exports the stable wire helpers. Task 1.1.3 should
extend that surface rather than introducing a parallel SSE module elsewhere.

### Relevant existing patterns

- `render_comment_frame("")` already renders the canonical empty SSE comment
  frame `:\n\n`. Task 1.1.3 should wrap or codify that output as the approved
  heartbeat helper instead of inventing a second comment-rendering rule.
- `render_event_frame` already handles deterministic field ordering and
  newline-safe `data:` output. The `stream_reset` helper should build on that
  primitive rather than formatting event frames manually.
- `EventId` and `ReplayCursor` show the current pattern for transport-focused
  SSE helpers: typed validation and small Actix-friendly utilities, not
  responders or lifecycle abstractions.
- The documentation set already has SSE sections in `docs/users-guide.md` and
  `docs/developers-guide.md`. Task 1.1.3 should extend those sections rather
  than creating parallel guides.

### Recommended target layout

Unless implementation reveals a simpler split, add:

```plaintext
src/sse/
  heartbeat.rs      # heartbeat policy and canonical heartbeat frame helper
  stream_reset.rs   # fixed replay-unavailable control event helper
```

Then update:

- `src/sse/mod.rs` for module declarations and re-exports.
- `src/lib.rs` for crate-root re-exports.

This keeps heartbeat policy and replay-reset signalling independently testable,
keeps files under the 400-line limit, and avoids overcrowding `frame.rs` with
higher-level control-plane helpers.

## Build and validation commands

Implementation work must capture every gate with `tee` and `set -o pipefail`.
Run the gates sequentially, never in parallel.

```bash
set -o pipefail && make check-fmt 2>&1 | tee /tmp/check-fmt-sse-heartbeat-stream-reset.out
set -o pipefail && make lint 2>&1 | tee /tmp/lint-sse-heartbeat-stream-reset.out
set -o pipefail && make test 2>&1 | tee /tmp/test-sse-heartbeat-stream-reset.out
set -o pipefail && make fmt 2>&1 | tee /tmp/fmt-sse-heartbeat-stream-reset.out
set -o pipefail && make markdownlint 2>&1 | tee /tmp/markdownlint-sse-heartbeat-stream-reset.out
set -o pipefail && make nixie 2>&1 | tee /tmp/nixie-sse-heartbeat-stream-reset.out
```

For this plan-only change, run only the documentation gates after editing the
Markdown files. Once implementation begins, rerun the full Rust and
documentation gate set before commit.

## Plan of work

The implementation should proceed in four stages. Each stage ends with a
validation step, and later stages should not begin until the current stage is
green.

### Stage A: lock the helper surface and any ADR refinements

Before writing code, confirm the smallest useful public API for this task. The
recommended direction is:

- one default heartbeat interval constant set to 20 seconds;
- one small typed heartbeat policy helper that supports explicit override;
- one canonical heartbeat frame helper that emits the approved empty comment
  frame without asking downstream code to call `render_comment_frame("")`
  directly;
- one fixed `stream_reset` helper that emits the standard event name and JSON
  payload without exposing generic application-event wiring.

This stage must also settle the few details ADR 001 leaves implicit:

- whether the explicit override path rejects `Duration::ZERO`;
- whether the heartbeat helper is entirely infallible or validates override
  inputs through a small error type;
- whether any public constants for the `stream_reset` event name or payload are
  needed, or whether a rendering helper alone is the smallest coherent surface.

If any of these choices changes or sharpens the normative contract, update ADR
001 before implementation proceeds.

Validation for Stage A:

- the user approves this plan;
- ADR 001 is ready to accept any needed clarifications;
- no scheduler, responder, or lifecycle abstraction has leaked into the
  proposed API.

### Stage B: implement heartbeat policy and rendering helpers

Create `src/sse/heartbeat.rs` with a narrow API that keeps policy data separate
from runtime scheduling.

Implementation expectations:

- expose the ADR default heartbeat interval of 20 seconds;
- provide an explicit override path through a small typed helper rather than
  requiring downstream code to copy raw `Duration` literals;
- keep scheduling responsibility with the application;
- provide one canonical heartbeat frame helper that reuses the existing
  comment-frame behaviour;
- keep the helper surface deterministic and cheap to call.

Unit test coverage in this stage should include at least:

- the default interval is 20 seconds;
- an explicit override is preserved exactly;
- unhappy-path validation for any rejected override values, if the approved API
  validates them;
- the heartbeat helper emits the exact approved wire frame;
- repeated rendering remains deterministic.

`rstest-bdd` is probably unnecessary here because the behaviour is interval
data plus deterministic frame output. If that remains true during
implementation, record that decision in the execplan and keep the tests as
`rstest` unit coverage.

Validation for Stage B:

- `make check-fmt`
- `make lint`
- targeted review of rendered output in tests

### Stage C: implement the fixed stream_reset helper

Create `src/sse/stream_reset.rs` as a narrow helper for the replay-unavailable
control event.

Implementation expectations:

- reuse `render_event_frame` rather than formatting SSE event text manually;
- emit the exact event name `stream_reset`;
- emit the exact JSON payload `{"reason":"replay_unavailable"}`;
- avoid adding any generic event-name or arbitrary JSON-payload helper for
  applications.

Unit test coverage in this stage should include at least:

- the helper renders the exact expected frame;
- the output is deterministic across repeated calls;
- any public constants added for the event name or payload remain aligned with
  the rendered frame.

If a minimal scenario-level test for "replay cannot resume, so the helper emits
the standard reset event" materially improves clarity, add one `rstest-bdd`
behavioural test. If not, document why focused unit tests are sufficient.

Validation for Stage C:

- `make check-fmt`
- `make lint`
- `make test`

### Stage D: documentation, roadmap, and final evidence capture

After code and tests are green, update the documentation set:

1. `docs/adr-001-shared-sse-wire-contract-for-wildside-and-corbusier.md`
   records any design decisions taken during implementation, especially the
   explicit override semantics and the chosen helper surface.
2. `docs/users-guide.md` documents the default heartbeat policy, the override
   path, and the standard `stream_reset` helper with concise examples. It must
   also state explicitly that this crate still does not provide heartbeat
   scheduling or a convenience responder.
3. `docs/developers-guide.md` documents the internal module split, the decision
   to reuse frame helpers instead of duplicating wire formatting, and the
   boundary that keeps timers and replay orchestration out of scope.
4. `docs/roadmap.md` marks task 1.1.3 done only after all implementation and
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
2. The approved surface remains wire-only and excludes any scheduler or
   convenience responder.
3. Any ADR clarification needed for override validation or helper shape has
   been accepted before code lands.

## Validation and acceptance

Task 1.1.3 is done when all of the following are true:

- `src/sse/` exports a shared heartbeat policy helper and a shared
  `stream_reset` helper that remain transport-focused.
- The default heartbeat interval is 20 seconds and the public API exposes an
  explicit override path.
- The `stream_reset` helper emits the exact ADR-defined event name and payload.
- Unit tests cover happy paths, unhappy paths, and relevant edge cases with
  `rstest`.
- Any behavioural tests added with `rstest-bdd` are justified by scenario
  clarity; otherwise the execplan explicitly records why they were not needed.
- `make check-fmt`, `make lint`, `make test`, `make fmt`,
  `make markdownlint`, and `make nixie` all pass.
- ADR 001, `docs/users-guide.md`, and `docs/developers-guide.md` reflect the
  approved helper behaviour.
- `docs/roadmap.md` marks 1.1.3 complete.

## Idempotence and recovery

The implementation should be additive and safe to rerun. If a stage fails,
repair the issue and rerun that stage's validation commands. No rollback should
be necessary unless the attempted API expands beyond the approved wire-helper
scope.

## Surprises & discoveries

- 2026-04-10: the current repository already contains the completed 1.1.2 SSE
  work (`render_event_frame`, `render_comment_frame`, and cache-control
  helpers), so task 1.1.3 can build directly on live framing primitives rather
  than adding new formatting logic.
- 2026-04-10: the current public docs already describe generic comment-frame
  heartbeats, so task 1.1.3 needs to document the normative 20-second policy
  and the dedicated helper surface rather than re-explaining SSE comments in
  general.
- 2026-04-10: no `execplans` skill was available in this session, so this plan
  was drafted using the repository's existing execplan pattern and local
  documentation guidance.
- 2026-04-11: `make fmt` depends on `mdformat-all`, which shells out to `fd`.
  This environment lacked `fd` on `PATH`, so validation used a temporary
  session-local `fd` shim to satisfy the existing Make target without changing
  repository code.

## Decision log

- 2026-04-10: recommend a split between `heartbeat.rs` and `stream_reset.rs`
  so interval policy and replay-reset signalling stay independently testable
  and below the repository's 400-line file guidance.
- 2026-04-10: recommend keeping task 1.1.3 strictly below the scheduler and
  responder boundary so downstream applications retain ownership of connection
  lifecycle and timer orchestration.
- 2026-04-10: recommend implementing `stream_reset` as a fixed helper that
  reuses `render_event_frame` and does not broaden into a generic
  application-event surface.
- 2026-04-11: approved `HeartbeatPolicy::new(Duration)` as the explicit
  override path and rejected `Duration::ZERO` with a dedicated validation error
  so the shared helper surface does not silently encode "disable heartbeats".
- 2026-04-11: kept coverage at the `rstest` unit-test layer because the landed
  behaviour is typed interval validation plus deterministic frame rendering;
  `rstest-bdd` would add ceremony without clarifying additional runtime
  behaviour.

## Outcomes & retrospective

- Landed `src/sse/heartbeat.rs` with:
  - `DEFAULT_HEARTBEAT_INTERVAL` set to 20 seconds;
  - `HeartbeatPolicy` with `Default` and validated explicit overrides;
  - `render_heartbeat_frame()` as the canonical empty-comment heartbeat helper.
- Landed `src/sse/stream_reset.rs` with:
  - `STREAM_RESET_EVENT_NAME`;
  - `STREAM_RESET_REPLAY_UNAVAILABLE_PAYLOAD`;
  - `render_stream_reset_frame()` built on `render_event_frame`.
- Updated crate-root and `src/sse/mod.rs` re-exports so downstream callers can
  use the helpers directly from `actix_v2a`.
- Updated ADR 001, the user's guide, the developer's guide, and the roadmap to
  describe the heartbeat override semantics and the fixed `stream_reset`
  surface.
- Passed the mandated quality gates with `tee` logs:
  - `make fmt`
  - `make check-fmt`
  - `make lint`
  - `make test`
  - `make markdownlint`
  - `make nixie`
