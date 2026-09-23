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
the report in explicit upload mode from the one step that binds the token,
behind exactly `env.CS_ACCESS_TOKEN != '' && github.ref == 'refs/heads/main'`,
in a concurrency group keyed on the evaluated ref and event that never cancels
a run in progress.

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
- A dispatch measures without advancing the baseline, and cannot displace a
  pending push.
- Adding a workflow that touches CodeScene, runs ratcheted coverage on a push,
  or changes the coverage selection fails the contract, which names the clause.
