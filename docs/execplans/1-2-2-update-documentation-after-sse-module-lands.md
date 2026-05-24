# Update documentation after the SSE module lands

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
 `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`, `Decision Log`,
and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Status: COMPLETE

## Purpose / big picture

Roadmap task 1.2.2 closes the documentation loop for the shared Server-Sent
Events (SSE) helper work from Architecture Decision Record (ADR) 001. The code
and contract-proof tasks have already landed in the earlier roadmap sequence:
tasks 1.1.1 through 1.1.3 built the shared wire-helper surface, and task 1.2.1
proved the contract with unit and behavioural coverage.

This task does not implement new SSE behaviour. It records the completed SSE
milestone in the older Wildside import execution plan, removes implementation
time environment-specific path references from finished documentation, updates
the documentation index and roadmap, and validates the final documentation set.
After this change, a maintainer should be able to read the documentation
without seeing sibling-checkout paths presented as normative references, while
still preserving enough provenance to understand where the imported Wildside
work came from.

Success is observable when:

1. `docs/execplans/import-components-from-wildside.md` records that the shared
   SSE milestone is complete and that its path-cleanup milestone was executed.
2. Environment-specific references such as sibling Wildside checkout paths no
   longer appear in normative sections of public or maintainer documentation.
3. `docs/contents.md` indexes this plan and uses canonical upstream wording for
   Wildside provenance.
4. `docs/roadmap.md` marks task 1.2.2 done only after the documentation
   closure and validation evidence are in place.
5. `make fmt`, `make markdownlint`, `make nixie`, `make check-fmt`,
   `make lint`, and `make test` pass with logs captured through `tee`.

This plan must be approved before implementation begins.

## Constraints

- Do not implement new SSE functionality in this task. The implementation
  scope is documentation, living-plan maintenance, validation evidence, branch
  publishing, and pull request preparation.
- Preserve the wire-only boundary from ADR 001. Documentation must not imply
  that `actix-v2a` owns event-store persistence, stream routing, authorization,
  heartbeat scheduling, or application payload schemata.
- Treat ADR 001 as the normative shared SSE wire contract unless the
  implementation pass discovers an actual conflict. If the closure task changes
  any contract claim, update ADR 001 in the same commit and record the decision
  in this plan.
- Keep provenance, but remove machine-specific and checkout-specific paths from
  normative prose. Prefer canonical upstream repository URLs, repository
  relative paths inside this crate, or short prose descriptions.
- Local sibling-checkout path references are allowed only in a clearly labelled
  historical or planning-time provenance section, and only if removing them
  would make the old plan less intelligible.
- Keep `docs/users-guide.md` changes conditional. Update it only if the closure
  audit finds public API wording, examples, or behaviour summaries that no
  longer match the landed SSE helpers.
- Keep `docs/developers-guide.md` changes conditional. Update it only if the
  closure audit finds maintainer-facing module, testing, or quality-gate
  guidance that no longer matches the landed SSE work.
- Use `leta` for code navigation if implementation discovers a need to inspect
  Rust symbols. Use ordinary text search for Markdown phrase audits and path
  reference sweeps.
- Use the `rust-router` skill only to route unexpected Rust-facing findings.
  The expected task is documentation-only, so no follow-on Rust language skill
  is expected unless code changes become unavoidable.
- Use the `hexagonal-architecture` skill as a boundary checklist, not as a
  mandate to reorganize files. The documentation should keep domain and adapter
  concerns separated where it describes the SSE helper surface.
- Signpost the relevant documentation for implementers:
  `docs/roadmap.md`, `docs/execplans/import-components-from-wildside.md`,
  `docs/adr-001-shared-sse-wire-contract-for-wildside-and-corbusier.md`,
  `docs/contents.md`, `docs/users-guide.md`, `docs/developers-guide.md`,
  `docs/documentation-style-guide.md`,
  `docs/rust-testing-with-rstest-fixtures.md`,
  `docs/reliable-testing-in-rust-via-dependency-injection.md`,
  `docs/rust-doctest-dry-guide.md`, and
  `docs/complexity-antipatterns-and-refactoring-strategies.md`.
- Run repository gates sequentially. Do not run formatting, linting, or tests
  in parallel.
- Capture long command output with `tee` under `/tmp`, using the current branch
  name in the log filename.
- Use `coderabbit review --agent` after each major implementation milestone
  that produces reviewable documentation changes, and clear all actionable
  concerns before moving to the next milestone.
- Commit small, focused changes after their relevant gates pass. Do not commit
  changes that fail the required gates.

## Tolerances

- Scope: if implementation requires non-documentation Rust code changes, stop
  and request approval before continuing. Narrow doctest or example fixes in
  documentation count as documentation changes; library code changes do not.
- Contract drift: if ADR 001, `docs/users-guide.md`, `docs/developers-guide.md`,
  and the landed `src/sse/` API disagree on an observable behaviour, stop after
  recording the conflict and ask which document or implementation should be
  authoritative.
- Path cleanup: after the scrub pass, the path sweep command shown in
  Milestone 1 must show zero hits outside explicitly labelled planning-time
  provenance notes. Any remaining unlabelled hit blocks closure.
- Historical preservation: if removing local paths would erase important
  import provenance from the Wildside plan, move the reference into an
  explicitly labelled provenance section rather than deleting it silently.
- Documentation gates: if `make fmt`, `make markdownlint`, or `make nixie`
  fails on changed documentation, fix the documentation before continuing.
- Full gates: if `make check-fmt`, `make lint`, or `make test` fails because
  of this branch's changes, fix the branch. If a failure is demonstrably
  unrelated and pre-existing, record the evidence and ask for direction before
  marking the roadmap item done.
- CodeRabbit: if `coderabbit review --agent` reports actionable concerns,
  address them or record why they are false positives before proceeding.
- Iterations: if the same gate or CodeRabbit concern remains unresolved after
  three focused fix attempts, stop and document the blocker in the Decision Log.

## Risks

- Risk: over-scrubbing the Wildside import plan removes useful provenance.
  Severity: medium. Likelihood: medium. Mitigation: replace normative local
  paths with canonical upstream references, and keep any necessary local
  checkout detail only in a labelled historical section.
- Risk: the roadmap could be checked off while the older import plan remains
  stale. Severity: medium. Likelihood: medium. Mitigation: make roadmap
  completion the final documentation edit, after the import plan, contents
  index, path sweep, gates, and CodeRabbit review are complete.
- Risk: documentation says the SSE module is broader than it is. Severity:
  high. Likelihood: low. Mitigation: use ADR 001, the user guide, and the
  developer guide to preserve the wire-helper boundary and explicitly exclude
  event-store, routing, authorization, and scheduling logic.
- Risk: a documentation-only task may skip Rust validation and miss doctest or
  lint drift introduced by edited examples. Severity: medium. Likelihood: low.
  Mitigation: run the documentation gates required by the roadmap and the full
  Rust gates requested for this feature before final commit.
- Risk: external protocol references change over time. Severity: low.
  Likelihood: medium. Mitigation: cite stable primary or ecosystem references
  only for context, not as a replacement for ADR 001. The useful references for
  this plan are the WHATWG HTML Server-Sent Events section and the `actix-sse`
  docs.rs page as prior art for Actix-facing SSE helpers.

## Repository orientation

The current repository already contains the landed SSE helper module under
`src/sse/`, with crate-root re-exports in `src/lib.rs`. The public guide in
`docs/users-guide.md` documents `EventId`, `ReplayCursor`, `Last-Event-ID`
header extraction, frame rendering, heartbeat helpers, `stream_reset`, and live
event-stream cache headers. The maintainer guide in `docs/developers-guide.md`
documents the `src/sse/` module layout, framework adapter boundary, and the
current `rstest`, `rstest-bdd`, and `proptest` testing split.

The older import plan at `docs/execplans/import-components-from-wildside.md` is
still marked `Status: IN PROGRESS`. It contains the historical milestones that
imported pagination, idempotency, shared error handling, OpenAPI fragments, and
the initial SSE contract decision. Its Milestone 6 already requires a final
documentation scrub for environment-specific paths. Roadmap item 1.2.2 is the
task that should execute that clean-up and then close the plan honestly.

ADR 001 remains the normative SSE contract. It says that `actix-v2a` provides
wire-only helpers for browser-compatible Server-Sent Events, including
validated identifiers, `Last-Event-ID` parsing, frame rendering, cache headers,
a configurable heartbeat policy with a 20-second default, and the standard
`stream_reset` event. It also says that application event stores, retention
policies, authorization, and endpoint routing remain outside this crate.

External context gathered during planning confirms two useful reference points:
the WHATWG HTML specification defines the `text/event-stream` format, the
`Last-Event-ID` header, UTF-8 decoding, field processing, and comment
heartbeats; the `actix-sse` crate demonstrates prior art for Actix Web SSE
responders while also reinforcing that this crate deliberately stops short of
owning responder lifecycle.

## Implementation plan

Use this shared path sweep helper anywhere this plan says to run the path sweep:

```bash
path_sweep() {
  LOCAL_WILDSIDE_PATH='../wildside'/'backend'
  LOCAL_HOME_PREFIX='/home'/'leynos'
  LOCAL_PROJECTS_PREFIX='/data/leynos'/'Projects'
  LOCAL_WORKTREE_SEGMENT='work''trees'
  PATH_PATTERN="${LOCAL_PROJECTS_PREFIX}|${LOCAL_WILDSIDE_PATH}"
  PATH_PATTERN="${PATH_PATTERN}|${LOCAL_HOME_PREFIX}|${LOCAL_WORKTREE_SEGMENT}"
  rg -n "${PATH_PATTERN}" "$@"
}
```

### Milestone 1: audit the documentation closure surface

Start by checking the branch and the current documentation state:

```bash
git branch --show-current
git status --short --branch
SSE_PATTERN='1\\.2\\.2|SSE|Server-Sent|Last-Event-ID|stream_reset|heartbeat'
path_sweep docs README.md
rg -n "${SSE_PATTERN}" \
  docs/roadmap.md docs/contents.md docs/execplans \
  docs/users-guide.md docs/developers-guide.md
```

Read these files before editing:

- `docs/roadmap.md`
- `docs/execplans/import-components-from-wildside.md`
- `docs/adr-001-shared-sse-wire-contract-for-wildside-and-corbusier.md`
- `docs/contents.md`
- `docs/users-guide.md`
- `docs/developers-guide.md`
- `docs/documentation-style-guide.md`

The milestone is complete when this plan's `Progress` section records the audit
findings and the path-reference sweep has a known baseline. Do not mark roadmap
task 1.2.2 done in this milestone.

### Milestone 2: update the Wildside import plan

Edit `docs/execplans/import-components-from-wildside.md` so it reflects the
current completed state without pretending the historical implementation path
never existed.

Required edits:

1. Change `Status: IN PROGRESS` to `Status: COMPLETE` only when the remaining
   documentation closure work is actually complete. During this milestone,
   leave it as `IN PROGRESS` until the path scrub and gates have passed.
2. Update `Purpose / big picture`, `Repository orientation`, `Constraints`,
   `Tolerances`, `Risks`, milestone text, and outcomes so normative references
   use canonical upstream Wildside wording or repository-relative paths instead
   of local Wildside checkout paths.
3. Add or update a clearly labelled planning-time provenance note if any local
   sibling-checkout paths must remain for historical intelligibility.
4. Add a dated `Progress` entry for roadmap task 1.2.2 beginning and later for
   completion.
5. Add a `Decision Log` entry that states the path-normalization rule:
   canonical upstream URLs for provenance, repository-relative paths for this
   crate, and labelled historical notes for any remaining local checkout
   references.
6. Update `Outcomes & Retrospective` to state that the SSE roadmap sequence
   from 1.1.1 through 1.2.1 has landed, and that task 1.2.2 closed the
   documentation and path-cleanup loop.

Run the path sweep after editing this file:

```bash
path_sweep docs/execplans/import-components-from-wildside.md
```

The milestone is complete when the import plan accurately records the SSE
completion state and contains no unlabelled environment-specific paths.

Run CodeRabbit for this milestone after the import-plan edit is staged or
otherwise reviewable:

```bash
coderabbit review --agent
```

Address actionable feedback before continuing.

### Milestone 3: update the documentation index and contract-adjacent docs

Edit `docs/contents.md` so it indexes this new plan and replaces any
implementation-time Wildside path wording with canonical upstream phrasing.
Keep the existing SSE execplan entries stable and add an entry for
`docs/execplans/1-2-2-update-documentation-after-sse-module-lands.md`.

Review ADR 001. Edit it only if its context still presents a local checkout
path as normative or if the closure audit finds a contract statement that no
longer matches the landed module. The expected edit, if needed, is a wording
change from local checkout language to canonical upstream Wildside provenance.

Review `docs/users-guide.md`. Edit it only if public API examples or behaviour
summaries are stale. If edited, keep examples aligned with the crate-root
exports and the existing SSE helpers.

Review `docs/developers-guide.md`. Edit it only if the internal module layout,
test suite description, validation guidance, or boundary language is stale. If
edited, keep the framework adapter rule explicit: `actix_adapter` is the only
SSE module that should import Actix Web types.

Run the broad path sweep:

```bash
path_sweep docs README.md
```

The milestone is complete when all changed documentation is internally
consistent, this execplan is discoverable from `docs/contents.md`, and the path
sweep has no unlabelled implementation-time references.

Run CodeRabbit again:

```bash
coderabbit review --agent
```

Address actionable feedback before continuing.

### Milestone 4: validate documentation and full repository gates

Run all required gates sequentially with `tee` logs. Use the branch name in the
log filenames, so results are easy to find after context compaction:

```bash
BRANCH=1-2-2-update-documentation-after-sse-module-lands
set -o pipefail && make fmt | tee "/tmp/fmt-actix-v2a-${BRANCH}.out"
set -o pipefail && make markdownlint \
  | tee "/tmp/markdownlint-actix-v2a-${BRANCH}.out"
set -o pipefail && make nixie | tee "/tmp/nixie-actix-v2a-${BRANCH}.out"
set -o pipefail && make check-fmt \
  | tee "/tmp/check-fmt-actix-v2a-${BRANCH}.out"
set -o pipefail && make lint | tee "/tmp/lint-actix-v2a-${BRANCH}.out"
set -o pipefail && make test | tee "/tmp/test-actix-v2a-${BRANCH}.out"
```

Expected results:

- `make fmt` completes and leaves no unintended formatting changes.
- `make markdownlint` passes on the documentation set.
- `make nixie` passes all Mermaid validation.
- `make check-fmt` reports no Rust formatting drift.
- `make lint` passes Clippy, Rustdoc, and repository lint policy.
- `make test` passes unit, behavioural, property, integration, and doctest
  coverage already present in the repository.

If any gate changes files, inspect the diff, record the change in `Progress`,
and rerun the affected gate before moving on.

### Milestone 5: mark roadmap completion and close the branch

After Milestones 2 through 4 are complete and CodeRabbit has no unresolved
actionable concerns, make the final documentation edits:

1. Set `Status: COMPLETE` in
   `docs/execplans/import-components-from-wildside.md` if not already done.
2. Add final gate evidence to that plan and to this plan's `Progress` and
   `Outcomes & Retrospective` sections.
3. Mark roadmap task 1.2.2 done in `docs/roadmap.md`.

Run the minimal final confirmation:

```bash
git diff --check
path_sweep docs README.md
```

If the final roadmap edit or status update touches Markdown formatting, rerun:

```bash
BRANCH=1-2-2-update-documentation-after-sse-module-lands
set -o pipefail && make fmt | tee "/tmp/fmt-actix-v2a-${BRANCH}.out"
set -o pipefail && make markdownlint \
  | tee "/tmp/markdownlint-actix-v2a-${BRANCH}.out"
set -o pipefail && make nixie | tee "/tmp/nixie-actix-v2a-${BRANCH}.out"
```

Commit using the `commit-message` skill's file-based `git commit -F` workflow.
Push the branch with upstream tracking:

```bash
git push -u origin 1-2-2-update-documentation-after-sse-module-lands
```

Create or update a draft pull request. The title must include the roadmap item
number as `(1.2.2)`, and the summary must mention this execplan document. The
description must include a `## References` section containing the Lody session
link produced from:

```bash
echo ${LODY_SESSION_ID}
```

## Validation and evidence capture

The implementation must record the exact gate outcomes in this plan and in the
Wildside import plan where relevant. Use `/tmp` only for logs and scratch
output. Do not use `/tmp` as a build target.

The required validation suite for the completed feature is:

- `make fmt`
- `make markdownlint`
- `make nixie`
- `make check-fmt`
- `make lint`
- `make test`
- `coderabbit review --agent`
- `git diff --check`
- the environment-specific path sweep shown in Milestone 1

The expected final path sweep output is empty, unless this plan or the import
plan contains a clearly labelled planning-time provenance note. Any allowed hit
must be recorded in `Decision Log` with the reason it remains.

## Approval gates

Implementation must not begin until all of these are true:

1. The user explicitly approves this plan.
2. The branch is named `1-2-2-update-documentation-after-sse-module-lands`.
3. The plan PR is open as a draft for review.

After approval, implementation proceeds milestone by milestone within the
tolerances above. Silence is not approval.

## Progress

- [x] 2026-05-19: Loaded the `leta`, `rust-router`, `execplans`,
  `firecrawl-mcp`, `commit-message`, `pr-creation`, and
  `hexagonal-architecture` skills needed for this planning task.
- [x] 2026-05-19: Created the Leta workspace for this repository.
- [x] 2026-05-19: Confirmed the starting branch was not `main` and renamed it
  to `1-2-2-update-documentation-after-sse-module-lands`.
- [x] 2026-05-19: Used a Wyvern agent team to inspect the roadmap, import
  plan, ADR, contents index, user guide, and developer guide for planning gaps.
- [x] 2026-05-19: Used Firecrawl to review current SSE protocol and prior-art
  references: WHATWG HTML Server-Sent Events and the `actix-sse` docs.rs page.
- [x] 2026-05-19: Drafted this execution plan.
- [x] 2026-05-20: Received explicit user approval to proceed with
  implementation from this plan.
- [x] 2026-05-20: Reconfirmed the branch name and clean upstream-tracking work
  tree before editing.
- [x] 2026-05-20: Re-ran the closure audit. The only required guide-facing
  changes are in `docs/execplans/import-components-from-wildside.md`, this
  plan, and the roadmap; `docs/users-guide.md` and `docs/developers-guide.md`
  already describe the landed SSE helper boundary accurately.
- [x] 2026-05-20: Normalized the old Wildside import plan's provenance
  wording, recorded SSE closure progress, updated ADR 001 and
  `docs/contents.md`, and scrubbed the stale absolute worktree path from
  `docs/execplans/portwildsidepagination.md`.
- [x] 2026-05-20: Ran the path sweep over `docs` and `README.md`; it returned
  no hits for the configured environment-specific path patterns.
- [x] 2026-05-20: Ran `coderabbit review --agent` for the first
  documentation-closure milestone; it completed with zero findings.
- [x] 2026-05-20: Passed the first documentation gate set:
  `make fmt`, `make markdownlint`, and `make nixie`, with logs under `/tmp`
  using the branch-specific filenames from this plan.
- [x] 2026-05-20: Committed the first documentation-closure milestone as
  `90c7dc1` (`Normalize SSE documentation provenance`).
- [x] 2026-05-20: Passed the full repository gate set:
  `make check-fmt`, `make lint`, and `make test`, with branch-specific logs
  under `/tmp`. The test gate ran 162 nextest tests and 26 doctests.
- [x] 2026-05-20: Marked `docs/execplans/import-components-from-wildside.md`
  complete and marked roadmap task 1.2.2 done after the full gate set passed.
- [x] 2026-05-20: Passed the final documentation gate set after the roadmap and
  import-plan closure edits: `make fmt`, `make markdownlint`, and `make nixie`.
- [x] 2026-05-20: Re-ran `coderabbit review --agent` for the final closure
  diff after a recoverable rate-limit wait; it completed with zero findings.
- [x] 2026-05-20: Committed the closure edits as `f134e2d`, pushed to the
  remote branch, and updated draft pull request #30 at
  <https://github.com/leynos/actix-v2a/pull/30>.

## Surprises & Discoveries

- 2026-05-19: The landed repository already has public and maintainer SSE
  documentation in `docs/users-guide.md` and `docs/developers-guide.md`, so
  task 1.2.2 should avoid rewriting those guides unless the closure audit finds
  concrete drift.
- 2026-05-19: `docs/execplans/import-components-from-wildside.md` still had
  many local Wildside checkout references in normative sections even though its
  own Milestone 6 says those paths should be removed before closure.
- 2026-05-19: The import plan's outcomes already claim Milestone 6 shipped, but
  the file is still `Status: IN PROGRESS` and still contains unscoped local
  path references. The implementation must reconcile that inconsistency rather
  than simply checking the roadmap box.
- 2026-05-19: No separate SSE design document exists under `docs/`; ADR 001
  plus the user's and developer's guides carry the current design and usage
  documentation.
- 2026-05-20: The broad path sweep found an unrelated absolute worktree path in
  `docs/execplans/portwildsidepagination.md`. It was part of an orientation
  transcript, not SSE content, but it still violated the completed
  documentation-set cleanup requirement.

## Decision Log

- 2026-05-19: Treat task 1.2.2 as documentation closure, not as functional SSE
  work. Rationale: roadmap tasks 1.1.1 through 1.2.1 already built and proved
  the helper surface, while 1.2.2 names plan and documentation updates.
- 2026-05-19: Keep remaining local checkout paths only as explicitly labelled
  historical provenance if needed. Rationale: the old import plan should remain
  intelligible, but finished documentation should not depend on one developer's
  sibling directory layout.
- 2026-05-19: Include the full Rust gate suite in the implementation plan even
  though the expected change is documentation-only. Rationale: the user asked
  for `make check-fmt`, `make lint`, and `make test` to succeed, and edited
  Rust examples or doctests can affect those gates.
- 2026-05-19: Use WHATWG HTML as the protocol reference and `actix-sse` as
  ecosystem prior art only. Rationale: ADR 001 remains the source of truth for
  this crate's deliberately narrower wire-helper contract.
- 2026-05-20: Scrub environment-specific path references from the final
  documentation set, not only from the SSE and Wildside import documents.
  Rationale: the roadmap item names the execplan closing milestone, and that
  milestone requires the finished documentation set to avoid implementation
  machine paths.

## Outcomes & Retrospective

Completed outcome: the documentation closure work scrubbed the configured
environment-specific path patterns from `docs` and `README.md`, recorded the
landed SSE helper sequence in the Wildside import plan, marked roadmap task
1.2.2 done, and updated draft pull request #30 for reviewer inspection. The
work stayed documentation-only; `docs/users-guide.md` and
`docs/developers-guide.md` did not need changes because their landed SSE helper
sections already matched the current crate boundary.

Validation passed with branch-specific logs under `/tmp`: `make fmt`,
`make markdownlint`, `make nixie`, `make check-fmt`, `make lint`, and
`make test`. The test gate ran 162 nextest tests and 26 doctests. CodeRabbit
reported zero findings for the first documentation milestone and zero findings
for the final closure diff after a recoverable rate-limit wait.

## External references

- WHATWG HTML Standard, Server-sent events:
  <https://html.spec.whatwg.org/multipage/server-sent-events.html>
- `actix-sse` crate documentation on docs.rs:
  <https://docs.rs/actix-sse/latest/actix_sse/>
