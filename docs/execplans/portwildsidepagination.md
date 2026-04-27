# Port Wildside pagination documentation hardening

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

## Purpose / big picture

Wildside commit `9d6b7655e3ad1a666a0e96beebd0a27b0d139388` published
stronger pagination documentation, documentation-invariant tests, and several
related test cleanups. `actix-v2a` already contains the shared pagination
primitives imported from Wildside, but its local documentation and tests do not
yet carry the full ordering, limit, error-mapping, and scope-boundary guidance
from that later Wildside commit.

The goal is to port the parts of that commit that improve the reusable
`actix-v2a` library without importing Wildside application code. Success is
observable when a maintainer can read `src/pagination/mod.rs` and
`docs/users-guide.md` to understand how to integrate cursor pagination, run a
dedicated documentation-invariant BDD suite, and see the same repository gates
pass with the new tests and documentation in place.

Do not begin implementation from this draft until the user explicitly approves
the plan. The planning phase may create and commit this document, but the code
and documentation port itself starts only after approval.

## Constraints

- Keep `actix-v2a` transport- and persistence-neutral. Do not add Diesel,
  Wildside domain, repository, route, or service concepts.
- Preserve the current public pagination API unless the user approves a public
  API change. The existing surface includes `Cursor`, `CursorError`,
  `Direction`, `PageParams`, `PageParamsError`, `Paginated`,
  `PaginationLinks`, `DEFAULT_LIMIT`, `MAX_LIMIT`, `PAGE_PARAM_LIMIT`, and
  `PAGE_PARAM_CURSOR`.
- Do not add new external dependencies. The current crate already has
  `base64`, `serde`, `serde_json`, `thiserror`, `url`, `utoipa`, `rstest`, and
  `rstest-bdd`, which are enough for the planned port.
- Keep every Rust source file at or below 400 lines. If added tests would push
  `tests/pagination_bdd.rs` or any module over that limit, split the test
  support into a focused sibling file.
- Use `actix-v2a` error codes when documenting HTTP mapping. Wildside's
  `invalid_cursor` and `invalid_page_params` envelope codes are not present in
  this crate; this crate currently exposes `ErrorCode::InvalidRequest` and
  `ErrorCode::InternalError` for those classes of failures.
- Documentation and comments must use en-GB Oxford spelling, while preserving
  external API spellings such as `Serialize`, `Deserialize`, `OpenAPI`, and
  `base64url`.
- Follow repository workflow: prefer Makefile targets, capture long command
  output through `tee` under `/tmp`, run gates sequentially, and commit with a
  message file via `git commit -F`.

## Tolerances (exception triggers)

- Scope: stop and escalate if implementation requires changes to more than 12
  repository files or more than 450 net lines, excluding generated formatting
  changes to this plan.
- Interface: stop and escalate if any public function, type, enum variant, or
  constant must be renamed, removed, or made public.
- Dependencies: stop and escalate if a new crate, tool, or non-Makefile gate is
  required.
- Semantics: stop and escalate if the port reveals that `actix-v2a` runtime
  pagination behaviour differs from the Wildside documentation in a way that
  cannot be fixed without changing public API behaviour.
- Tests: stop and escalate if `make check-fmt`, `make lint`, `make test`,
  `make markdownlint`, or `make nixie` still fails after two focused fix
  attempts for the same failure.
- Ambiguity: stop and present options if OpenAPI pagination documentation would
  require choosing between exposing a new reusable schema and documenting only
  endpoint-local `#[utoipa::path]` query parameters.

## Risks

- Risk: Wildside's error-mapping table names envelope codes that do not exist
  in `actix-v2a`.
  Severity: medium.
  Likelihood: high.
  Mitigation: port the client-versus-server classification, not the literal
  code strings. Document `CursorError::InvalidBase64`,
  `CursorError::Deserialize`, `CursorError::TokenTooLong`, and
  `PageParamsError::InvalidLimit` as `ErrorCode::InvalidRequest`, and
  `CursorError::Serialize` as `ErrorCode::InternalError`.

- Risk: The upstream commit contains many non-pagination test and domain
  refactors that are tempting but not suitable for this library.
  Severity: medium.
  Likelihood: high.
  Mitigation: limit implementation to pagination docs, pagination tests,
  documentation-invariant tests, and directly relevant guide updates.

- Risk: Adding a shared BDD fixture module could duplicate too much of the
  existing `tests/pagination_bdd.rs` setup.
  Severity: low.
  Likelihood: medium.
  Mitigation: extract only the state and helper functions that are needed by
  both existing pagination scenarios and the new documentation scenarios.

- Risk: Markdown formatting and Rustdoc examples can fail after prose changes.
  Severity: medium.
  Likelihood: medium.
  Mitigation: run `make fmt`, `make markdownlint`, `make nixie`, `make lint`,
  and `make test`, and keep examples executable unless they intentionally
  demonstrate pseudocode or endpoint-local integration.

## Progress

- [x] (2026-04-26 23:31Z) Loaded repository instructions, the `leta`,
  `rust-router`, `execplans`, and `commit-message` skills, and confirmed the
  branch is `feat/portwildsidepagination`.
- [x] (2026-04-26 23:31Z) Used a wyvern agent team for read-only
  reconnaissance: one agent analysed Wildside commit `9d6b7655`, and one agent
  mapped the relevant `actix-v2a` pagination, OpenAPI, query DTO, test, and
  gate surfaces.
- [x] (2026-04-26 23:31Z) Cloned Wildside commit `9d6b7655` into
  `/tmp/wildside-9d6b7655-read` and reviewed the upstream pagination docs,
  feature file, BDD documentation tests, and shared test fixture.
- [x] (2026-04-26 23:31Z) Drafted this ExecPlan for user review.
- [x] (2026-04-26 23:33Z) Added this plan to `docs/contents.md` and validated
  the planning-only documentation change with `make check-fmt`,
  `make markdownlint`, and `make nixie`.
- [-] (2026-04-26 23:33Z) Attempted `make fmt`; `cargo +nightly fmt --all`
  completed, but the target failed because `mdformat-all` is not installed in
  this environment.
- [x] (2026-04-27 00:46Z) Received explicit user approval to proceed with the
  implementation as set out in this ExecPlan.
- [x] (2026-04-27 00:46Z) Completed Stage A documentation changes by expanding
  pagination module docs and adding pagination integration guidance to
  `docs/users-guide.md`.
- [x] (2026-04-27 00:46Z) Validated Stage A with `make check-fmt`,
  `make lint`, `make markdownlint`, and `make nixie`.
- [x] (2026-04-27 00:52Z) Completed Stage B by adding
  `tests/features/pagination_documentation.feature` and
  `tests/pagination_documentation_bdd.rs`.
- [x] (2026-04-27 00:52Z) Completed Stage C by adding
  `CursorError::Serialize` unit coverage and `PageParams` limit-boundary
  coverage around `MAX_LIMIT`.
- [x] (2026-04-27 00:52Z) Ran the full implementation gates:
  `make check-fmt`, `make lint`, `make test`, `make markdownlint`, and
  `make nixie`.
- [x] (2026-04-27 00:52Z) Implemented all approved documentation and test
  changes.
- [x] (2026-04-27 00:52Z) Committed the completed implementation and plan
  update.
- [x] (2026-04-27 01:00Z) Fixed the post-turn hook environment issue where
  `cargo` was absent from `PATH` by making Cargo recipes prepend the existing
  `$HOME/.cargo/bin` directory while still invoking `cargo` by name.

## Surprises & Discoveries

- Observation: `actix-v2a` already contains the imported pagination foundation,
  including direction-aware cursors, `PageParams` normalization, and
  `PaginationLinks::from_request`.
  Evidence: `src/pagination/mod.rs`, `src/pagination/cursor.rs`,
  `src/pagination/params.rs`, `src/pagination/envelope.rs`, and
  `tests/pagination_bdd.rs` already cover these behaviours.
  Impact: The port should harden documentation and coverage, not re-import the
  pagination implementation.

- Observation: Wildside commit `9d6b7655` changed 71 files, but most changes
  are Wildside-specific domain, adapter, example-data, and script fixes.
  Evidence: `git show --stat 9d6b7655` lists broad backend modules such as
  catalogue, offline bundles, Overpass enrichment, users, WebSocket sessions,
  `bun.lock`, and example-data generator modules.
  Impact: These are excluded from the `actix-v2a` port unless a later user
  request asks for a separate test-fixture refactor.

- Observation: Wildside's pagination documentation says OpenAPI schema
  generation is out of scope for its standalone pagination crate, while
  `actix-v2a` already has reusable `utoipa` schema fragments for other shared
  envelopes.
  Evidence: Wildside `backend/crates/pagination/src/lib.rs` documents OpenAPI
  schema generation as a consumer responsibility; local
  `src/openapi/schemas.rs` contains `ErrorCodeSchema`, `ErrorSchema`, and
  `ReplayMetadataSchema` only.
  Impact: The plan should document endpoint-local pagination query parameters
  rather than introduce a new reusable pagination OpenAPI schema by default.

- Observation: `make fmt` is not fully runnable in this environment because
  `mdformat-all` is missing.
  Evidence: `/tmp/fmt-actix-v2a-feat-portwildsidepagination.out` records
  `make: mdformat-all: No such file or directory` after
  `cargo +nightly fmt --all` completed.
  Impact: The planning commit can still be gated by `make check-fmt`,
  `make markdownlint`, and `make nixie`, but the implementation phase should
  either install `mdformat-all` or record the same environmental limitation.

- Observation: Adding a pagination "Error mapping" heading to
  `docs/users-guide.md` conflicted with the existing SSE "Error mapping"
  heading under Markdown lint's duplicate-heading rule.
  Evidence: `make markdownlint` reported `MD024/no-duplicate-heading` for
  `docs/users-guide.md`.
  Impact: The pagination heading was renamed to "Pagination error mapping",
  preserving the intended content while keeping the guide lint-clean.

- Observation: A self-contained documentation-invariant BDD suite avoided
  pushing the existing `tests/pagination_bdd.rs` file over the repository's
  400-line limit.
  Evidence: `tests/pagination_bdd.rs` was already 367 lines before Stage B, and
  the new scenarios fit cleanly in `tests/pagination_documentation_bdd.rs`.
  Impact: No shared fixture extraction was needed for this port, keeping the
  change smaller than the optional common-fixture path in the plan.

- Observation: The first isolated run of the new BDD suite caught a borrowed
  `Err(...)` comparison and a macro-expanded unused-braces warning in the
  `world` fixture.
  Evidence: `cargo test --test pagination_documentation_bdd` failed before
  those local issues were fixed, then passed after the targeted correction.
  Impact: The focused red/green run validated the new suite before the full
  repository gates.

- Observation: The post-turn hook runs with a reduced `PATH` that did not
  include the existing Cargo installation directory.
  Evidence: The hook reported `make: cargo: No such file or directory` while
  running `make check-fmt lint`; `env PATH=/usr/bin:/bin make check-fmt lint`
  reproduced the same environment shape.
  Impact: The Makefile now prepends `$HOME/.cargo/bin` to Cargo recipe `PATH`
  values while continuing to call `cargo` by name. No shim, install step, or
  recreated system script was introduced.

## Decision Log

- Decision: Treat this as a documentation and test hardening port, not an
  implementation import.
  Rationale: The implementation was already imported by
  `docs/execplans/import-components-from-wildside.md`; the later Wildside
  commit mainly improves crate-level explanation and verification.
  Date/Author: 2026-04-26 23:31Z / Codex.

- Decision: Exclude Wildside domain, example-data, Bun/script, and broad
  backend test-fixture changes from this port.
  Rationale: Those changes target Wildside's application architecture and do
  not map cleanly to a small shared Actix helper library.
  Date/Author: 2026-04-26 23:31Z / Codex.

- Decision: Adapt Wildside's error-mapping guidance to `actix-v2a`'s current
  `ErrorCode` enum.
  Rationale: Porting literal `invalid_cursor` or `invalid_page_params` codes
  would require public API changes and contradict this plan's compatibility
  constraint.
  Date/Author: 2026-04-26 23:31Z / Codex.

- Decision: Document endpoint-local `utoipa::path` query parameters in the user
  guide instead of adding reusable pagination schema wrappers.
  Rationale: The plan's compatibility constraint forbids public API expansion
  without approval, and pagination response schemata depend on each endpoint's
  concrete item type.
  Date/Author: 2026-04-27 00:46Z / Codex.

- Decision: Keep the documentation-invariant BDD suite self-contained rather
  than extracting shared pagination BDD fixtures.
  Rationale: The existing pagination BDD file was already near the line-count
  limit, and the new suite needed only small state and helper functions.
  Date/Author: 2026-04-27 00:52Z / Codex.

- Decision: Make Cargo discovery recipe-local, not global, in the Makefile.
  Rationale: The hook needed the existing Cargo binary directory on `PATH`, but
  the repository should still call `cargo` directly and avoid shims or
  generated wrapper scripts.
  Date/Author: 2026-04-27 01:00Z / Codex.

## Outcomes & Retrospective

This plan is complete. The port delivered pagination module and user-guide
documentation for ordering requirements, limit normalization, error mapping,
scope boundaries, Actix query extraction, and endpoint-local OpenAPI parameter
documentation. It also added documentation-invariant BDD coverage plus unit
tests for `CursorError::Serialize` and `PageParams` behaviour around
`MAX_LIMIT`.

The implementation deliberately did not add public API, new dependencies, or
Wildside application-specific code. Full validation passed with
`make check-fmt`, `make lint`, `make test`, `make markdownlint`, and
`make nixie`. `make fmt` remains partially blocked in this environment because
`mdformat-all` is not installed, but `cargo fmt --all` and
`make check-fmt` both succeeded.

After completion, the post-turn hook exposed a reduced-`PATH` environment that
could not find `cargo`. The Makefile now prepends the existing Cargo bin
directory for Cargo recipes and the existing Bun bin directory for
`markdownlint-cli2`, without installing or creating any replacement scripts.
The reduced-`PATH` reproductions passed for `make check-fmt lint` and
`make markdownlint`.

## Context and orientation

The current repository is the `actix_v2a` Rust crate. Pagination code lives
under `src/pagination/`:

- `src/pagination/mod.rs` provides module-level documentation and re-exports.
- `src/pagination/cursor.rs` defines `Cursor<Key>`, `Direction`, and
  `CursorError`.
- `src/pagination/params.rs` defines `PageParams`, `PageParamsError`,
  `DEFAULT_LIMIT`, `MAX_LIMIT`, `PAGE_PARAM_LIMIT`, and `PAGE_PARAM_CURSOR`.
- `src/pagination/envelope.rs` defines `Paginated<T>` and `PaginationLinks`.

User-facing crate documentation currently lives in `docs/users-guide.md`, and
maintainer guidance lives in `docs/developers-guide.md`. Behavioural tests use
`rstest-bdd` in `tests/pagination_bdd.rs` with Gherkin scenarios in
`tests/features/pagination.feature` and
`tests/features/direction_aware_cursors.feature`. OpenAPI component tests live
in `tests/openapi_schemas_bdd.rs`.

The relevant upstream Wildside files from commit `9d6b7655` are:

- `backend/crates/pagination/src/lib.rs`
- `backend/crates/pagination/src/cursor.rs`
- `backend/crates/pagination/src/envelope.rs`
- `backend/crates/pagination/src/params.rs`
- `backend/crates/pagination/tests/common.rs`
- `backend/crates/pagination/tests/features/pagination_documentation.feature`
- `backend/crates/pagination/tests/pagination_bdd.rs`
- `backend/crates/pagination/tests/pagination_documentation_bdd.rs`
- `docs/developers-guide.md`

The upstream commit also contains useful local test fixes, including
`CursorError::Serialize` coverage and `PageParams` boundary coverage around
`MAX_LIMIT`. Those are portable because the same error variants and constants
exist in `actix-v2a`.

## Plan of work

Stage A updates documentation without changing behaviour. Expand
`src/pagination/mod.rs` with the Wildside ordering requirements, default and
maximum limit contract, consumer-owned database ordering responsibility,
property-based testing responsibility, error-mapping guidance adapted to
`ErrorCode::InvalidRequest` and `ErrorCode::InternalError`, and scope
boundaries. Add short module-level context to `src/pagination/cursor.rs`,
`src/pagination/params.rs`, and `src/pagination/envelope.rs` covering cursor
encoding security, `PageParams` deserialization normalization, and link query
parameter preservation. Add a concise pagination section to
`docs/users-guide.md` with an `actix_web::web::Query<PageParams>` integration
example and endpoint-local `utoipa` query-parameter note. Do not add new public
schema wrappers in this stage.

Stage B adds documentation-invariant coverage. Create
`tests/features/pagination_documentation.feature` with scenarios for default
limits, maximum limits, zero-limit rejection, invalid base64 cursor errors,
invalid JSON cursor errors, token-too-long errors, and human-readable display
strings. Create `tests/pagination_documentation_bdd.rs` to implement those
scenarios. If shared state duplication becomes material, create
`tests/pagination_common.rs` or `tests/common/pagination.rs` and move only
shared `World`, `FixtureKey`, and limit setup helpers there, then update
`tests/pagination_bdd.rs` to use that helper module. Keep this extraction small
and purely test-local.

Stage C adds unit test hardening copied in spirit from Wildside. In
`src/pagination/cursor.rs`, add a test-only key type whose `Serialize`
implementation fails, then assert `Cursor::encode` returns
`CursorError::Serialize` with a useful message. In
`src/pagination/params.rs`, replace the single oversized-limit test with
focused `rstest` cases showing `MAX_LIMIT - 1` passes through unchanged,
`MAX_LIMIT` passes through unchanged, and values above `MAX_LIMIT` clamp to
`MAX_LIMIT`. Keep existing zero-limit and deserialization coverage.

Stage D validates and commits. Run formatting and quality gates sequentially
with `tee` logs in `/tmp`, inspect failures from the logs, make only focused
fixes, and commit the approved implementation with a file-based commit
message. If Stage A through C require more scope than the tolerances allow,
update this ExecPlan and stop for direction.

## Concrete steps

Run all commands from repository root:

```sh
pwd
git branch --show
git status --short
```

Expected orientation:

```plaintext
/home/leynos/.lody/repos/github---leynos---actix-v2a/worktrees/5e4b6a0b-5a00-4e8c-b862-6b6b7fe27926
feat/portwildsidepagination
```

After user approval, edit these files in order:

1. `src/pagination/mod.rs`
2. `src/pagination/cursor.rs`
3. `src/pagination/envelope.rs`
4. `src/pagination/params.rs`
5. `docs/users-guide.md`
6. `docs/developers-guide.md`, only if the shared test-layout guidance from
   Wildside is still useful after local test changes
7. `tests/features/pagination_documentation.feature`
8. `tests/pagination_documentation_bdd.rs`
9. `tests/pagination_bdd.rs`, only if shared fixture extraction is needed

Format and validate with logs named by action and branch:

```sh
make fmt 2>&1 | tee /tmp/fmt-actix-v2a-feat-portwildsidepagination.out
make check-fmt 2>&1 | tee /tmp/check-fmt-actix-v2a-feat-portwildsidepagination.out
make lint 2>&1 | tee /tmp/lint-actix-v2a-feat-portwildsidepagination.out
make test 2>&1 | tee /tmp/test-actix-v2a-feat-portwildsidepagination.out
make markdownlint 2>&1 | tee /tmp/markdownlint-actix-v2a-feat-portwildsidepagination.out
make nixie 2>&1 | tee /tmp/nixie-actix-v2a-feat-portwildsidepagination.out
git status --short
```

If all gates pass, inspect the diff and commit with a message file:

```sh
git diff -- src/pagination tests docs
git status --short
COMMIT_MSG_DIR="$(mktemp -d)"
cat > "$COMMIT_MSG_DIR/COMMIT_MSG.md" << 'ENDOFMSG'
Harden pagination docs and invariants

Port the reusable pagination documentation hardening from Wildside while
adapting error-mapping guidance to the `actix-v2a` shared error codes.

Add documentation-invariant coverage for the documented limit, cursor, and
error contracts so future changes keep the prose and behaviour aligned.
ENDOFMSG
git commit -F "$COMMIT_MSG_DIR/COMMIT_MSG.md"
rm -rf "$COMMIT_MSG_DIR"
```

## Validation and acceptance

Acceptance for the implementation is all of the following:

- `src/pagination/mod.rs` explains ordering requirements, default and maximum
  limits, consumer-owned query ordering, property-based testing responsibility,
  error mapping, and scope boundaries.
- `docs/users-guide.md` contains a pagination section that shows how an Actix
  handler consumes `PageParams`, returns `Paginated<T>`, and documents
  endpoint-local OpenAPI query parameters without promising a reusable schema
  wrapper that does not exist.
- `tests/features/pagination_documentation.feature` and
  `tests/pagination_documentation_bdd.rs` verify the documented invariants.
- Unit tests cover `CursorError::Serialize` and `PageParams` behaviour around
  `MAX_LIMIT`.
- The gate commands `make check-fmt`, `make lint`, `make test`,
  `make markdownlint`, and `make nixie` pass. `make fmt` may modify files
  before the validation gates.

Quality method:

- Inspect `tee` logs under `/tmp` for each gate.
- Record any unexpected failures in `Surprises & Discoveries`.
- Update `Progress`, `Decision Log`, and `Outcomes & Retrospective` before the
  implementation commit.

## Idempotence and recovery

The implementation steps are additive and can be repeated. If formatting
changes more files than expected, inspect `git diff --stat` before continuing.
If a gate fails, read the matching `/tmp` log, make one focused fix, rerun that
gate, and then continue with the remaining gates. If the same gate fails after
two focused attempts, stop under the tolerance rules.

The Wildside scratch clone in `/tmp/wildside-9d6b7655-read` is read-only
reference material and can be deleted at any time. The canonical source remains
GitHub commit `9d6b7655e3ad1a666a0e96beebd0a27b0d139388`.

## Artifacts and notes

Wildside commit summary:

```plaintext
9d6b765 Publish crate pagination docs, tests, and exec plan (4.1.3) (#333)
71 files changed, 3727 insertions(+), 1924 deletions(-)
```

Portable pagination files from that commit:

```plaintext
backend/crates/pagination/src/lib.rs
backend/crates/pagination/src/cursor.rs
backend/crates/pagination/src/envelope.rs
backend/crates/pagination/src/params.rs
backend/crates/pagination/tests/common.rs
backend/crates/pagination/tests/features/pagination_documentation.feature
backend/crates/pagination/tests/pagination_documentation_bdd.rs
backend/crates/pagination/tests/pagination_bdd.rs
```

Excluded Wildside-only surfaces:

```plaintext
backend/src/domain/*
backend/src/inbound/http/*
backend/src/inbound/ws/*
backend/src/outbound/*
crates/example-data/*
bun.lock
scripts/check-overrides-parity.test.mjs
docs/backend-roadmap.md
docs/wildside-backend-architecture.md
```

## Interfaces and dependencies

The port must keep using the existing pagination interfaces:

```rust
pub struct Cursor<Key> { /* existing private fields */ }
pub enum CursorError {
    Serialize { message: String },
    InvalidBase64 { message: String },
    TokenTooLong { max_len: usize },
    Deserialize { message: String },
}
pub enum Direction { Next, Prev }
pub struct PageParams { /* existing private fields */ }
pub enum PageParamsError { InvalidLimit }
pub struct PaginationLinks {
    pub self_: String,
    pub next: Option<String>,
    pub prev: Option<String>,
}
pub struct Paginated<T> {
    pub data: Vec<T>,
    pub limit: usize,
    pub links: PaginationLinks,
}
```

The port must not introduce a public `RawPageParams`, `PaginationParamsSchema`,
or endpoint-level OpenAPI wrapper unless the user explicitly approves that
change after reviewing this draft.

Revision note: Initial draft created after reviewing Wildside commit
`9d6b7655e3ad1a666a0e96beebd0a27b0d139388` and the current `actix-v2a`
pagination surface. The plan was then updated with planning-gate evidence and
the local `mdformat-all` limitation. The remaining work is blocked on explicit
approval because this plan recommends implementation changes beyond the
planning phase.

Revision note: Implementation began on 2026-04-27 after explicit user approval.
The plan status changed from `DRAFT` to `IN PROGRESS`; the remaining work is to
ship the approved documentation and test changes, run the gates, and record the
outcome.

Revision note: Stage A documentation changes were completed and validated on
2026-04-27. The remaining work is Stage B documentation-invariant BDD coverage,
Stage C unit test hardening, final validation, and completion notes.

Revision note: Stages B and C were completed on 2026-04-27, full validation
passed, and the plan status changed to `COMPLETE`. The only remaining workflow
step is to commit this final implementation and plan update.

Revision note: Post-completion hook hardening was added on 2026-04-27 after a
reduced-`PATH` hook could not find `cargo`. This does not change the pagination
implementation; it keeps the documented validation commands runnable in the
hook environment.
