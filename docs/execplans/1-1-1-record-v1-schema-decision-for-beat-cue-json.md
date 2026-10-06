# Record the v1 schema decision for BeatCue JSON

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DRAFT

## Purpose / big picture

BeatCue needs one settled v1 JSON contract before later cue-extraction work
adds command-line output, library return types, inference adapters, fixtures,
and stable snapshots. This plan records the schema technology decision for
BeatCue JSON so that future work can implement one coherent public package
surface instead of revisiting serialization choices in every adapter.

The intended implementation outcome is documentation-first. It should add or
update an Architecture Decision Record (ADR) that ratifies `msgspec.Struct` as
the v1 schema technology, then signpost that ADR from the design document,
roadmap, user's guide, developer's guide, and documentation index where those
documents exist. Success is observable when a maintainer can read the ADR and
understand why `msgspec` is suitable for library callers, command-line
interface (CLI) JSON output, round-trip validation, and stable snapshot tests.

This plan is not approved for implementation yet. Implementation must wait for
explicit user approval.

## Constraints

These are hard invariants. Violation requires escalation, not workaround.

- Implementation must not begin until this ExecPlan is explicitly approved.
- The implementation must follow `docs/roadmap.md` item 1.1.1 as the governing
  delivery unit. In this worktree, the visible `docs/roadmap.md` currently
  describes `actix-v2a` Server-Sent Events (SSE) work rather than the supplied
  BeatCue roadmap text, so this mismatch must be resolved before code or
  permanent documentation changes proceed.
- The requested `docs/beatcue-technical-design.md` document is not present in
  this worktree at plan-drafting time. The implementing agent must either
  obtain it, switch to the correct branch or repository, or record explicit
  user approval to proceed from the prompt text alone.
- The requested `.rules/python-*.md` files are not present in this worktree at
  plan-drafting time. The implementing agent must not claim to have applied
  rules that are unavailable.
- The plan must signpost and use the `execplans` skill for execution-plan
  maintenance and the `hexagonal-architecture` skill for boundary checks.
- The implementation must preserve a hexagonal boundary: domain schema types
  may depend on schema and validation libraries, but must not depend on CLI,
  filesystem, network, user interface, or inference-service adapters.
- The ADR must explain why `msgspec.Struct` is the selected v1 BeatCue JSON
  schema technology and which decisions remain deferred.
- Documentation must use en-GB-oxendict spelling and follow
  `docs/documentation-style-guide.md`.
- If code is added later, every new module must have module-level purpose
  documentation and public APIs must have user-facing documentation.
- Vidai Mock must be used for behavioural testing of inference services when
  inference-service behaviour is exercised. This item is expected to be a
  schema decision record; direct inference-service tests are out of scope unless
  the implementation adds or changes an inference boundary.
- The final implementation must run the requested gates sequentially:
  `make check-fmt`, `make typecheck`, `make lint`, and `make test`. If
  documentation changes include Markdown, also run `make markdownlint` and
  `make nixie`.
- On completion of the approved implementation, mark roadmap item 1.1.1 as
  done in the governing roadmap for the BeatCue workstream.
- Branch and pull-request work must use
  `1-1-1-record-v1-schema-decision-for-beat-cue-json` as the branch name and
  include `(1.1.1)` in the draft pull-request title.

## Tolerances

These thresholds define when autonomous implementation must stop and ask for
direction.

- Scope: if implementation requires changes outside `docs/`, test fixtures, or
  minimal schema scaffolding, stop and ask whether this is still a decision
  record task.
- Repository fit: if the BeatCue roadmap, design document, or Python rules are
  still absent after checking the current branch and remote tracking branch,
  stop before implementing and ask whether to switch repositories, switch
  branches, or proceed with prompt-derived source material.
- Interface: if the implementation needs a public API signature beyond
  documenting the selected schema shape, stop and ask for approval.
- Dependencies: if adding `msgspec`, `pytest`, `pytest-bdd`, `syrupy`,
  `hypothesis`, or Vidai Mock configuration is required for this item, stop and
  confirm whether implementation should expand from documentation into package
  scaffolding.
- Test stack: if the repository remains Rust-only, do not force Python test
  tools into the project merely to satisfy generic BeatCue guidance. Stop and
  resolve the repository mismatch first.
- Iterations: if one quality gate fails after three focused fix attempts on the
  same root cause, stop, document the blocker, and ask for direction.
- Ambiguity: if there are two plausible ADR locations or schema-source-of-truth
  locations with different maintenance consequences, present the options before
  writing permanent documentation.

## Risks

- Risk: the current worktree may not be the BeatCue repository or branch.
  Severity: high. Likelihood: high. Mitigation: verify branch, remotes, and
  governing docs before implementation; stop if BeatCue source documents remain
  absent.

- Risk: `msgspec.Struct` could be ratified without enough detail to guide
  later library and CLI contracts. Severity: medium. Likelihood: medium.
  Mitigation: require the ADR to cover library callers, CLI JSON output,
  round-trip validation, stable snapshots, rejected alternatives, and deferred
  decisions.

- Risk: schema technology decisions can leak adapter concerns into domain
  types. Severity: medium. Likelihood: medium. Mitigation: apply the
  `hexagonal-architecture` skill and keep CLI, inference, filesystem, and
  network concerns outside the domain schema module.

- Risk: adding tests for a documentation-only ADR could create brittle
  low-value checks. Severity: low. Likelihood: medium. Mitigation: use
  Markdown, link, and documentation gates for ADR-only changes; add unit,
  behavioural, snapshot, or property tests only when executable schema code or
  observable contracts are introduced.

## Progress

- [x] (2026-05-09T19:19:24Z) Loaded the `execplans`,
  `hexagonal-architecture`, and `leta` skills for planning context.
- [x] (2026-05-09T19:19:24Z) Spawned a Wyvern agent to inspect the roadmap,
  design, style, rule, and architecture documents without editing files.
- [x] (2026-05-09T19:19:24Z) Verified the current branch is
  `feat/beatcue-schema-adr-plan`, not the main branch.
- [x] (2026-05-09T19:19:24Z) Found that `docs/beatcue-technical-design.md` and
  `.rules/python-*.md` are absent in this worktree.
- [x] (2026-05-09T19:19:24Z) Found that the visible `docs/roadmap.md` describes
  `actix-v2a` SSE work rather than the supplied BeatCue roadmap item.
- [x] (2026-05-09T19:19:24Z) Drafted this ExecPlan.
- [x] (2026-05-09T19:29:00Z) Validated this plan-only change with
  `make check-fmt`, `make typecheck`, `make lint`, `make test`,
  `make markdownlint`, and `make nixie`.
- [x] (2026-05-09T19:34:00Z) Renamed the local branch to
  `1-1-1-record-v1-schema-decision-for-beat-cue-json`.
- [ ] Obtain explicit approval for this ExecPlan before implementation.
- [ ] Establish upstream tracking for
  `origin/1-1-1-record-v1-schema-decision-for-beat-cue-json` when pushing the
  branch.
- [ ] Implement the approved decision-record change.
- [ ] Run the required sequential quality gates.
- [ ] Commit the approved change after gates pass.
- [ ] Push the branch and create a draft pull request with `(1.1.1)` in the
  title and this ExecPlan named in the summary.

## Surprises & discoveries

- Observation: the requested BeatCue design document is not present.
  Evidence: `find .. -name beatcue-technical-design.md` returned no matches.
  Impact: implementation cannot truthfully cite sections 7, 13.1, 17, or 21
  until the correct document is available.

- Observation: the requested Python rule files are not present.
  Evidence: searching `.rules` failed because the directory does not exist in
  this worktree.
  Impact: implementation must either obtain the rules or document that the
  repository does not currently contain them.

- Observation: the repository appears to be the Rust `actix-v2a` crate, while
  the task describes a Python BeatCue package using `msgspec`, `pytest`,
  `pytest-bdd`, `syrupy`, and `hypothesis`.
  Evidence: `Makefile` delegates to Rust/Netsuke targets and `docs/roadmap.md`
  tracks SSE helpers for `actix-v2a`.
  Impact: the first implementation milestone must verify whether the branch or
  repository is correct before editing product documentation.

- Observation: network access to GitHub was unavailable in the default sandbox
  during branch discovery.
  Evidence: `git ls-remote --heads origin
  1-1-1-record-v1-schema-decision-for-beat-cue-json` failed with `Could not
  resolve host: github.com`.
  Impact: remote tracking, push, and draft pull-request creation may require
  elevated command execution.

## Decision log

- Decision: draft the ExecPlan with an explicit repository-fit blocker instead
  of inventing unavailable BeatCue design details.
  Rationale: the requested plan must be self-contained and evidence-based; the
  governing BeatCue documents are not available in this worktree.
  Date/Author: 2026-05-09T19:19:24Z / Codex.

- Decision: treat executable Python tests as conditional rather than mandatory
  for the ADR-only slice.
  Rationale: item 1.1.1 asks to record a schema technology decision. Unit,
  behavioural, snapshot, and property tests become mandatory when executable
  schema code, CLI output, or inference boundaries are introduced.
  Date/Author: 2026-05-09T19:19:24Z / Codex.

## Outcomes & retrospective

No implementation has started. The current outcome is a draft implementation
plan that identifies the missing BeatCue source documents, the repository
mismatch, the approval gate, and the quality gates required before a later
commit can be made. The plan-only change has passed the repository quality
gates listed in `Progress`.

## Context and orientation

The current working directory is:

```plaintext
/home/leynos/.lody/repos/github---leynos---actix-v2a/worktrees/bf1f1885-8772-44ad-80f0-b94c9c350cc8
```

The visible repository is `actix-v2a`, a Rust library crate. Its `Makefile`
delegates to Netsuke targets such as `make check-fmt`, `make typecheck`,
`make lint`, `make test`, `make markdownlint`, and `make nixie`.

The requested work item describes BeatCue JSON, a Python-oriented schema
decision using `msgspec.Struct`, `pytest`, `pytest-bdd`, `syrupy`, and
`hypothesis`. Those names do not match the visible repository's current source
and roadmap. A future implementing agent must first determine whether the
correct BeatCue branch is missing, whether this worktree is intentionally
repurposed for BeatCue planning, or whether the task was routed to the wrong
repository.

Important terms:

- `msgspec.Struct` is a Python data-structure base class from `msgspec` that
  supports fast typed JSON encoding, decoding, and validation.
- An ADR is an Architecture Decision Record: a narrow document that captures
  one accepted or proposed architectural decision, including context, options,
  outcome, and consequences.
- Hexagonal architecture separates domain logic from adapters. For BeatCue,
  schema types belong near the domain or application contract; CLI rendering,
  filesystem access, inference-service calls, and network wiring belong in
  adapters.
- Vidai Mock is the local inference-service simulator to use when tests need
  deterministic behaviour from model-provider boundaries.

## Plan of work

Stage A is repository reconciliation. Confirm the working branch and remote
state. Check whether `docs/beatcue-technical-design.md`, `.rules/python-*.md`,
and the BeatCue roadmap item appear after fetching the target branch
`origin/1-1-1-record-v1-schema-decision-for-beat-cue-json`. If they do not,
stop and ask whether to proceed in this repository from the prompt alone or to
switch to a different worktree. Do not edit product documentation during this
stage.

Stage B is documentation-source selection. If BeatCue documents are available,
read `docs/beatcue-technical-design.md` sections 7, 13.1, 17, and 21, the
governing roadmap entry, `docs/documentation-style-guide.md`,
`.rules/python-*.md`, `docs/complexity-antipatterns-and-refactoring-strategies.md`,
and any existing ADR index. Decide whether to create a new ADR, for example
`docs/adr-002-beatcue-json-v1-schema-technology.md`, or update an existing
BeatCue ADR if one already governs schema technology. If the repository still
contains only `actix-v2a` SSE documents, stop under the repository-fit
tolerance.

Stage C is the red-check stage. Before writing the final ADR, add or identify
the smallest failing validation that proves the missing documentation link or
schema decision is not yet present. For a documentation-only implementation,
this may be a Markdown link check, contents-file expectation, or review note
rather than executable Python tests. If executable schema scaffolding is
approved, add unit tests with `pytest` for `msgspec.Struct` encoding, decoding,
and validation; behavioural tests with `pytest-bdd` for user-visible JSON
contracts; `syrupy` snapshot tests for stable JSON output; and `hypothesis`
property tests for round-trip invariants.

Stage D is ADR implementation. Write the ADR following
`docs/documentation-style-guide.md`. The ADR should include `Status`, `Date`,
`Context and problem statement`, `Decision drivers`, `Options considered`,
`Decision outcome / proposed direction`, `Goals and non-goals`, `Known risks
and limitations`, and `Outstanding decisions` when relevant. The outcome must
state that BeatCue v1 JSON uses `msgspec.Struct`, and it must explain why this
fits library callers, CLI JSON output, round-trip validation, and stable
snapshot tests. It should compare at least `msgspec.Struct`, standard-library
`dataclasses` plus manual JSON handling, Pydantic models, and untyped
dictionaries.

Stage E is signposting. Update the BeatCue design document to point to the ADR
as the source of truth for v1 JSON schema technology. Update
`docs/contents.md` if a new ADR is added. Update `docs/users-guide.md` only if
library callers or CLI users now have visible schema or API guarantees to rely
on. Update `docs/developers-guide.md` or the relevant component architecture
document with internal schema conventions, such as where `msgspec.Struct`
types live, where adapters convert to or from them, and how fixtures and
snapshots should be maintained.

Stage F is validation and commit. Run formatting, type checking, linting, and
tests sequentially using `tee` logs in `/tmp`. Fix any failures without
expanding scope. When all required gates pass, mark roadmap item 1.1.1 done in
the BeatCue roadmap, commit the completed change with a descriptive message,
push the requested branch, and open a draft pull request whose title includes
`(1.1.1)` and whose summary mentions this ExecPlan.

## Concrete steps

Run these commands from the repository root.

Verify the branch and status:

```sh
git branch --show-current
git status --short
```

Expected result before implementation:

```plaintext
1-1-1-record-v1-schema-decision-for-beat-cue-json
```

If the branch has not yet been renamed, rename it and set the upstream after
confirming the remote branch:

```sh
git fetch origin
git branch -m 1-1-1-record-v1-schema-decision-for-beat-cue-json
git branch --set-upstream-to=origin/1-1-1-record-v1-schema-decision-for-beat-cue-json
```

Inspect the required BeatCue documents:

```sh
test -f docs/beatcue-technical-design.md
find .rules -name 'python-*.md' -maxdepth 1 -type f
```

If either command fails, stop and resolve the repository-fit tolerance before
implementation.

Run the required quality gates sequentially. Use branch-specific log names;
replace slashes in the branch name if the shell or filesystem requires it.

```sh
set -o pipefail && make check-fmt 2>&1 | tee /tmp/check-fmt-1-1-1-record-v1-schema-decision-for-beat-cue-json.out
set -o pipefail && make typecheck 2>&1 | tee /tmp/typecheck-1-1-1-record-v1-schema-decision-for-beat-cue-json.out
set -o pipefail && make lint 2>&1 | tee /tmp/lint-1-1-1-record-v1-schema-decision-for-beat-cue-json.out
set -o pipefail && make test 2>&1 | tee /tmp/test-1-1-1-record-v1-schema-decision-for-beat-cue-json.out
```

For Markdown documentation changes, also run:

```sh
set -o pipefail && make markdownlint 2>&1 | tee /tmp/markdownlint-1-1-1-record-v1-schema-decision-for-beat-cue-json.out
set -o pipefail && make nixie 2>&1 | tee /tmp/nixie-1-1-1-record-v1-schema-decision-for-beat-cue-json.out
```

Expected successful gate ending:

```plaintext
Finished
```

or the repository-specific equivalent successful Make/Netsuke completion line.

## Validation and acceptance

The plan is accepted when all of the following are true:

- The user has explicitly approved this ExecPlan or an updated revision.
- The repository-fit blocker is resolved: the BeatCue roadmap and design
  documents are available, or the user explicitly authorizes proceeding from
  the prompt text alone in this repository.
- The ADR states the selected v1 JSON schema technology as `msgspec.Struct`.
- The ADR explains suitability for library callers, CLI output, round-trip
  validation, and stable snapshots.
- The ADR identifies deferred decisions so later cue-extraction work does not
  infer unapproved contracts.
- The design document and relevant guide documents signpost the ADR.
- Roadmap item 1.1.1 is marked done only after the approved implementation and
  gates pass.
- `make check-fmt`, `make typecheck`, `make lint`, and `make test` pass.
- For documentation changes, `make markdownlint` and `make nixie` pass.
- If executable schema code is added, the relevant `pytest`, `pytest-bdd`,
  `syrupy`, and `hypothesis` coverage exists and passes.
- If inference-service behaviour is exercised, behavioural tests use Vidai
  Mock rather than a live provider.

## Idempotence and recovery

Documentation edits are safe to repeat when each stage checks current files
before adding new sections or links. If a new ADR filename is chosen and later
proves incorrect, prefer `git mv` before the first commit so links and history
stay coherent.

If a validation command fails, inspect its `/tmp` log, make the smallest
necessary correction, and rerun only the failed gate before continuing through
the remaining gates. If a command fails because the environment cannot access
network resources or a sandboxed path, request elevated execution rather than
changing project configuration or creating isolated caches.

Do not mark the roadmap item done until the implementation is complete and all
required gates pass. If implementation is abandoned because the repository is
wrong, leave the roadmap unchanged and record the blocker in this plan.

## Artifacts and notes

Initial planning evidence:

```plaintext
Original branch: feat/beatcue-schema-adr-plan
Current branch: 1-1-1-record-v1-schema-decision-for-beat-cue-json
Missing file: docs/beatcue-technical-design.md
Missing directory: .rules/
Visible roadmap: actix-v2a SSE helper roadmap, not BeatCue JSON schema roadmap
Wyvern review: confirmed the same repository/document mismatch
Validation: check-fmt, typecheck, lint, test, markdownlint, and nixie passed
```

## Interfaces and dependencies

The expected schema dependency for the eventual BeatCue implementation is
`msgspec`. The ADR should ratify `msgspec.Struct` as the base for v1 JSON
schema types. It should not require implementation code unless approval expands
this item beyond decision recording.

If schema code is approved in the same slice, the expected public shape is a
small domain-facing module containing typed `msgspec.Struct` records for
BeatCue JSON. CLI adapters should encode those records for output but should
not own the schema definitions. Inference adapters should convert provider
responses into domain/application types through explicit boundaries and should
be tested with Vidai Mock when behaviour depends on model-provider responses.

Revision note: initial draft created on 2026-05-09. It records the requested
BeatCue implementation plan, the missing BeatCue source documents in the
current worktree, the repository-fit blocker, and the approval gate before
implementation.
