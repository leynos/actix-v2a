# Architectural decision record (ADR) 005: `main` owns coverage publication

## Status

Accepted. Adopts concordat's CV-005, `main-owned-codescene-coverage`.

## Date

2026-09-23.

## Context and problem statement

Coverage was measured on pull requests and sent to CodeScene from the same
lane. Both halves of that failed quietly. A pull request from a fork cannot read
`secrets.CS_ACCESS_TOKEN`, so the upload was skipped for exactly the changes
that most need review, and CodeScene accepts an upload only for a branch it
analyses, which a pull request head is not. With CodeScene's coverage gates
switched off, its pull-request check mode now fails outright. A lane that holds
the token is also a lane a pull request's workflow edit could try to reach.

## Decision drivers

- Keep a failing coverage ratchet on every pull request.
- Keep the CodeScene credential, host, and CLI off every workflow a pull
  request can reach, directly or through a called workflow.
- Give the ratchet baseline exactly one writer, so every pull request is
  measured against `main`.
- Hold the split with an executable contract rather than a convention.

## Decision outcome

Two workflows split the work. The pull-request lane runs the shared
`generate-coverage` action with `with-ratchet: 'true'` and
`publish-artefact: 'false'`, and nothing else touches CodeScene. The publisher,
`coverage-main.yml`, runs on a push to `main` and on dispatch: it measures the
same pinned selection, which writes the ratchet baseline on a push, and uploads
the report in explicit upload mode, passing the secret as the action's
`access-token` input and guarded on exactly
`steps.codescene-token.outputs.available == 'true' && github.ref == 'refs/heads/main'`,
where a check step reports whether the secret is set and no step holds it in
its `env`, in a concurrency group keyed on the ref alone that never cancels a
run in progress. Runs never overlap and a newer trigger replaces an older
pending run, but GitHub does not promise to start runs in trigger order, so
commit order is not guaranteed; a manual re-run of an older run republishes
that commit's coverage but, its baseline cache key being run-keyed, replaces no
baseline unless the original run saved none.

`tests/coverage_workflows.rs` enforces the split: it reads every workflow a
pull request can reach as a closure through local reusable-workflow calls,
every workflow a push can start for second baseline writers, and every other
workflow for stray CodeScene access, and drives each rule against breaching
fixtures.

## Options considered

- Keep the upload on the pull-request lane. Rejected: forks never upload, heads
  are not analysed branches, and the lane must hold the token.
- Drop CodeScene coverage altogether. Rejected: the trunk figure is still
  wanted, and the ratchet needs a trunk-written baseline anyway.
- Protect the publisher with a deployment environment restricted to `main`.
  Deferred: it is a repository-settings change awaiting the owner's decision.
  Until then, a writer who dispatches an edited copy of the publisher on a
  branch could reach the token, as any writer already could by pushing a new
  workflow; the ref guard stops only unedited dispatches.

## Consequences

- A pull request's coverage is judged only by the ratchet; CodeScene sees
  `main`.
- A dispatch measures without advancing the baseline; one that replaces a
  pending push leaves the baseline a commit behind until the next push.
- Adding a workflow that touches CodeScene, runs ratcheted coverage on a push,
  or changes the coverage selection fails the contract, which names the clause.

## Addendum, 2026-09-29: the publisher runs on Ubicloud

`coverage-upload` moved from `ubuntu-latest` to `ubicloud-standard-2`, keeping
the estate's fork arm, which never applies on a push or a dispatch.

- **Why the publisher moves first.** Ubicloud's cache proxy is scoped by ref.
  A pull request's Ubicloud lane reads a warm main scope only when a main job
  on Ubicloud writes it, and the publisher is main's only writer. Moving it
  back to a hosted runner would silently return every Ubicloud pull request to
  a cold cache, so the placement contract holds the runner and ceiling to the
  file.
- **Ceilings.** An Ubicloud runner has no six-hour hosted cap, so each lane
  states its own, twice a measured warm Ubicloud run. `coverage-upload` is at 5
  minutes: its first Ubicloud main run took 2.4 minutes (run 36556920321),
  after a provisional 30 minutes. `build-test` is at 20 minutes: a warm
  standard-4 run took 9.6 minutes (run 36568765141).
- **`build-test` has moved.** It runs on `ubicloud-standard-4`, one main run
  after the publisher's move; on `ubicloud-standard-2` its uncached tool
  installs and lint alone reached a 30-minute ceiling (run 36565211532). Pull
  requests from forks stay on `ubuntu-latest` for both lanes and restore a
  hosted scope main no longer refreshes; fork pull requests are rare here and a
  second hosted writer would pay double on every main push.

The developers' guide section "Runner placement" records the operating rules.

## Addendum, 2026-09-29: the contract moved to a shared library

The contract that enforces this decision no longer lives in this repository.
`make test-workflow-contracts` runs `cv005-contracts check`, the shared
contract library in `leynos/shared-actions` (`packages/cv005-contracts`), from
a full commit pinned in the Makefile, and `.github/cv005.toml` holds this
repository's parameters. The clauses are unchanged, and the library's own suite
proves each one. The paragraphs above name the repository-local copy this
replaces.
