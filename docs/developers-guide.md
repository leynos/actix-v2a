# Developer's Guide

## Spelling policy

Run the spelling gate with:

```bash
netsuke build spelling
```

The `typos_config_builder_version` variable in the `Netsukefile` pins the
`typos-config-builder` release the gate runs (currently `v0.1.3`). Raise it
together with the regenerated `typos.toml`, never on its own.

The tracked `typos.toml` is regenerated on every run from the live shared
dictionary and the repository-specific `typos.local.toml` overlay. Never edit
generated entries by hand; add only narrow repository terminology to the
overlay. Because the dictionary is live, `typos.toml` must never be drift
checked in continuous integration.

The shared `typos-config-builder` CLI refreshes the estate dictionary into an
untracked local cache only when the authoritative copy is newer. A valid cache
remains usable when the network is unavailable. Quoted APIs and identifiers
retain their upstream spelling; put them in backticks or fenced code blocks
where practical rather than adding broad word-level exceptions.

This guide documents internal conventions, module structure, and quality gates
for contributors working on the `actix-v2a` crate. For repository-wide
orientation, start with [documentation contents](contents.md) and
[repository layout](repository-layout.md).

## Coverage publication

Coverage has two workflows, and the split is a contract (concordat's CV-005,
`main-owned-codescene-coverage`), not a convention.
[ADR 0005](adrs/adr-0005-main-owns-coverage-publication.md) records the
decision.

- `ci.yml` measures lcov coverage on every pull request with the shared
  `generate-coverage` action, `with-ratchet: 'true'` and
  `publish-artefact: 'false'`. A drop against the ratchet baseline fails the
  pull request. The lane holds no CodeScene credential, has no upload step, and
  never contacts CodeScene.
- `coverage-main.yml` runs on every push to `main` and on dispatch. It measures
  the same source with the same action, format, output path, and default
  baseline files. A push to `main` writes the ratchet baseline every pull
  request compares against; a dispatch reads it without advancing it. The lane
  then uploads the report to CodeScene in explicit upload mode. A check step
  reports whether the secret is set by evaluating
  `${{ secrets.CS_ACCESS_TOKEN != '' }}` into its output, and no step holds the
  token in its `env`, because the composite upload action would hand a step
  `env` to its nested `upload-artifact` and cache steps; the upload step passes
  the secret as its `access-token` input. The check runs earlier in the
  upload's own job, under no default shell, since a step's output is readable
  only there. The upload's `if:` is exactly
  `steps.codescene-token.outputs.available == 'true'` joined by `&&` to
  `github.ref == 'refs/heads/main'` (a dispatch can name any branch, and any
  further conjunct could only narrow, defeat, or invert the upload), and the
  workflow's concurrency group, exactly
  `${{ github.workflow }}-${{ github.ref }}` at every level, never cancels a
  run in progress, so a burst of merges cannot abandon a baseline write. Runs
  for `main` never overlap, and a newer trigger replaces an older pending run
  rather than queueing behind it. GitHub does not promise to start runs in
  trigger order, so this does not guarantee commit order. A manual re-run of an
  older run keeps its SHA and its run id: it republishes that commit's coverage
  to CodeScene, but replaces no ratchet baseline unless the original run saved
  none. The workflow answers exactly a push to `main` and `workflow_dispatch`,
  and the coverage selection both lanes run is pinned in the contract.

One known exception: a Dependabot pull request merged by the automerge workflow
with `GITHUB_TOKEN` fires no push event, so that merge is neither measured nor
uploaded until the next push to `main`; shared-actions #518 tracks the fix.
There is deliberately no `schedule` trigger to paper over it. Likewise, a
dispatch that replaces a pending push uploads the same or a newer commit, but
the ratchet baseline is saved only on a push, so it stays one commit behind
until the next push; shared-actions #518 covers that too.

The reasons are both quiet failures: a pull request from a fork cannot read the
secret, so an upload there is silently skipped, and CodeScene accepts an upload
only for a branch it analyses, which a pull request head is not.

`netsuke build test-workflow-contracts` enforces the split by running
`cv005-contracts check`, the shared contract library in `leynos/shared-actions`
(`packages/cv005-contracts`), from a full commit named by the
`cv005_contracts_ref` variable in the `Netsukefile`; CI runs it in a "Check the
CV-005 contracts" step. A fix to the rules is therefore a pin bump. The target
needs `uv`, which fetches the Python 3.13 the library runs under. The
repository's parameters are in `.github/cv005.toml`: its `repository` name and
the `[selection]` the baseline measures (`format`, `output-path` and
`with-ratchet`), which the publisher's generator must carry and every
pull-request lane must match. The library's own suite proves each rule refuses
the shape it exists to refuse, so this repository keeps no copy of the readers
or the refusal cases.

The rule covers every workflow a pull request can reach, following local
reusable-workflow calls transitively, and every other workflow too: only the
publisher may hold the token, reach a secret by a computed name, name the
CodeScene host, run the CLI or the uploader, or touch the retired
`CODESCENE_CLI_SHA256` variable. Workflows are read strictly: a duplicate key,
or a workflow declaring both `on` and `true`, is refused rather than silently
resolved, and a reading failure exits 2 rather than passing. The pull-request
surface is seeded by every event that runs a workflow for a pull request, and a
workflow a push starts, or one it calls, may run a ratcheted coverage step only
behind `if: github.event_name == 'pull_request'`, so the publisher stays the
baseline's only writer. When adding a workflow, keep CodeScene, `cs-coverage`,
and the token out of it unless it is the publisher; the library names the
clause a change breaks.

## Runner placement

`ci.yml`'s `build-test` runs on `ubicloud-standard-4` and `coverage-main.yml`'s
`coverage-upload`, main's only cache writer, on `ubicloud-standard-2`.
`runs-on` selects the class with the runner-selection expression, shown here for
`standard-2`:

```yaml
runs-on: >-
  ${{ (github.event.pull_request.head.repo.fork
  || github.event.pull_request.user.login == 'dependabot[bot]')
  && 'ubuntu-latest' || 'ubicloud-standard-2' }}
```

A pull request from a fork cannot obtain an Ubicloud runner, so it falls back to
`ubuntu-latest`. So does a pull request Dependabot opened: hosted minutes cost
nothing on a public repository, and Dependabot opens many pull requests. The
expression tests the pull request's author, not `github.actor`, because the
actor is whoever triggered the run, so a person pushing to a Dependabot branch
would flip the runner. A push and a dispatch have no pull request, so both
values are empty strings (falsy) and the expression selects Ubicloud; a
Dependabot pull request therefore builds cold on the hosted pool, and only
main's Ubicloud jobs write the cache.

The writer sits on Ubicloud because Ubicloud's cache proxy is scoped by ref. A
pull request's Ubicloud lane reads a warm main scope only when a main job on
Ubicloud writes it, so a lane can move to Ubicloud only after its main writer
has.

An Ubicloud runner is a self-hosted just-in-time runner, so GitHub's six-hour
cap for hosted jobs does not bound it and a hung job would hold a billable
runner. Every job whose `runs-on` can select Ubicloud therefore states its own
`timeout-minutes`: twice a measured warm Ubicloud run. `build-test` is at 20
minutes (a warm standard-4 run took 9.6 min, run 36568765141).
`coverage-upload` is at 40 minutes: its first Ubicloud main run took 2.4 min
(run 36556920321), but the test suite now executes Netsuke, and a run that
misses the Netsuke cache compiles `netsuke-build` from source. That added 7.7
min to a standard-4 `build-test` run (run 40045937287), so the ceiling allows
twice that on two vCPUs, doubled again. `build-test` is `standard-4` on a
measured shortfall: on `standard-2` the tool installs and lint alone took 30
minutes and the job reached its ceiling (run 36565211532). A fork's pull
request restores a hosted cache that main no longer refreshes; fork pull
requests are rare here, and a second hosted writer would pay double on every
main push.

`tests/runner_placement/placement_cases.rs` holds this to the files. It
evaluates the expression for a push or dispatch, a same-repository pull
request, a fork and a Dependabot pull request, rejects a literal label,
inverted arms, another label, another condition, a missing Dependabot arm and
an actor test, and asserts an exact inventory of the jobs that can land on
Ubicloud with their ceilings. A change that adds, removes or re-times such a
job fails it until the inventory is updated in the same commit.

## SSE module internals

The `src/sse/` module implements validated Server-Sent Events (SSE) wire-level
helpers per
[ADR 0001](adrs/adr-0001-shared-sse-wire-contract-for-wildside-and-corbusier.md).

### Module layout

```plaintext
src/sse/
  mod.rs               # Public re-exports and module documentation
  actix_adapter.rs     # Actix Web adapter functions for SSE domain helpers
  cache_control.rs     # Live event-stream cache policy helper
  event_id.rs          # EventId validated newtype and validation error
  frame.rs             # SSE event and comment frame rendering
  header.rs            # Framework-agnostic SseHeader type
  heartbeat.rs         # Heartbeat interval policy and canonical heartbeat frame
  replay_cursor.rs     # ReplayCursor type and Last-Event-ID header extraction
  stream_reset.rs      # Standard replay-unavailable control event helper
```

### Validation strategy

#### Forbidden character detection

The `EventId` validation scans for exactly three forbidden byte values:

- Carriage return: `0x0D` (U+000D)
- Line feed: `0x0A` (U+000A)
- NULL: `0x00` (U+0000)

These are the characters that would corrupt the SSE wire format per the WHATWG
HTML specification § 9.2.6. No other characters are forbidden by the wire
format.

The validation uses byte iteration (not char iteration) for efficiency, since
all three forbidden values are single-byte characters in UTF-8:

```rust
for byte in value_str.as_bytes() {
    if matches!(byte, b'\r' | b'\n' | b'\0') {
        return Err(EventIdValidationError::ForbiddenCharacter);
    }
}
```

#### Empty identifier handling

Empty identifiers are rejected because an empty `id:` field has special meaning
in the SSE specification: it resets the last event identifier rather than
setting one. An empty `Last-Event-ID` header value is treated as `Ok(None)` by
`extract_replay_cursor`, consistent with this specification behaviour.

#### Whitespace preservation

Unlike `IdempotencyKey`, which trims whitespace and validates UUID format,
`EventId` preserves leading and trailing whitespace verbatim. The identifier is
treated as an opaque string where whitespace may be meaningful to the
identifier generation strategy.

### Type relationships

- `EventId` — validated newtype wrapping `String`. Construction validates
  absence of CR, LF, and NULL.
- `ReplayCursor` — validated newtype wrapping `EventId`. Distinguishes "outgoing
  identifier for SSE `id:` line" from "identifier received from client
  reconnection header".
- `EventIdValidationError` — validation error enum with two variants:
  - `Empty` — identifier was empty
  - `ForbiddenCharacter` — identifier contained CR, LF, or NULL
- `ReplayCursorError` — replay cursor extraction error enum:
  - `Empty` — replay cursor value was empty after validation
  - `ForbiddenCharacter` — replay cursor contained CR, LF, or NULL
  - `InvalidHeader` — `Last-Event-ID` header was malformed (duplicate or
    non-UTF-8)
  - `variant_name()` — internal helper returning a stable structured-log field
    value for each variant
- `SseFrameError` — event/comment frame rendering error enum:
  - `EmptyEventName` — an explicit event name was provided but empty
  - `InvalidEventName` — event name contained CR, LF, or NULL
  - `InvalidData` — event payload contained NULL
  - `InvalidComment` — comment payload contained NULL
- `HeartbeatPolicy` — typed heartbeat interval wrapper with a 20-second default
  and explicit non-zero override path
- `HeartbeatPolicyError` — heartbeat policy validation error enum:
  - `ZeroInterval` — explicit override used `Duration::ZERO`

The `ReplayCursor` wrapping is semantic, not functional. Both types expose the
same string via `as_ref()` and `Display`. The distinction allows type
signatures to communicate intent: an `EventId` parameter documents "this
function emits SSE frames", while a `ReplayCursor` parameter documents "this
function handles client reconnection".

### Header extraction pattern

The `extract_replay_cursor` function follows the same single-header-or-error
pattern used by `extract_idempotency_key` in `src/idempotency/http.rs`:

```rust
let mut header_values = headers.get_all(LAST_EVENT_ID_HEADER);
let Some(header_value) = header_values.next() else {
    return Ok(None);  // Missing header is allowed
};
if header_values.next().is_some() {
    // Duplicate header fails; adapter logs before returning.
    return Err(ReplayCursorError::InvalidHeader);
}
```

Key differences from idempotency key extraction:

- Empty header values return `Ok(None)` (not an error) because the SSE
  specification treats empty `id:` fields as a reset of the last event
  identifier.
- Header values are not trimmed, consistent with treating identifiers as opaque
  strings.

### Frame rendering rules

The framing helpers stay deliberately small:

- `render_event_frame` renders complete frames with deterministic field order:
  `id:`, then `event:`, then one or more `data:` lines, then a blank line.
- `render_comment_frame` renders one or more comment lines followed by a blank
  line.
- `event_name: None` is the only supported way to express the default browser
  `message` event. `Some("")` is rejected.
- `data` and comment payloads normalize `\r`, `\n`, and `\r\n` into logical
  line breaks. This preserves embedded blank lines and trailing newlines while
  keeping the rendered wire text valid.
- NULL is rejected in `data` and comment payloads because it cannot be
  represented safely in the shared wire contract.

The implementation intentionally uses pure string rendering helpers rather than
an Actix responder, so downstream applications keep control of stream
lifecycle, heartbeats, authorization, and replay orchestration.

### Heartbeat and stream-reset helpers

Task 1.1.3 adds two higher-level helpers on top of `frame.rs`:

- `render_heartbeat_frame` delegates to `render_comment_frame("")` so the
  approved heartbeat frame stays aligned with the lower-level comment framing
  rules.
- `render_stream_reset_frame` delegates to `render_event_frame` with the fixed
  event name `stream_reset` and the fixed payload
  `{"reason":"replay_unavailable"}`.

The heartbeat policy remains data-only. `HeartbeatPolicy::default()` exposes
the ADR default of 20 seconds, while `HeartbeatPolicy::new` accepts explicit
non-zero overrides and rejects `Duration::ZERO`. This keeps "disable
heartbeats" outside the shared wire contract and avoids smuggling scheduler
policy into the crate.

### Cache-control policy

`apply_event_stream_cache_control` mutates a `Vec<SseHeader>` in place and sets:

- `Cache-Control: no-cache, no-store, must-revalidate`

For Actix Web callers, use `apply_actix_event_stream_cache_control` (in
`src/sse/actix_adapter.rs`), which accepts an Actix `HeaderMap` and delegates
to the domain function after converting header types.

### Framework adapter layer

The `actix_adapter` module is the sole location within `src/sse/` permitted to
import `actix_web` types. It exposes two public functions:

- `apply_actix_event_stream_cache_control(headers: &mut HeaderMap)` — inserts
  the canonical `Cache-Control` value into an Actix response `HeaderMap`.
<!-- markdownlint-disable-next-line MD013 -->
- `extract_actix_replay_cursor(headers: &HeaderMap) -> Result<Option<ReplayCursor>, ReplayCursorError>`
  — collects all `Last-Event-ID` values from an Actix request `HeaderMap`,
  converts each to a domain `SseHeader`, and delegates to
  `extract_replay_cursor`.

The `SseHeader` type (in `src/sse/header.rs`) is a plain name/value pair with
case-insensitive name comparison. Domain functions accept `&[SseHeader]` or
`&mut Vec<SseHeader>` to remain framework-independent.

### Tracing

`extract_replay_cursor` is a pure query and must not emit tracing events. The
Actix adapter boundary logs replay cursor extraction failures with structured
`tracing::error!` events before returning typed errors to callers. Two internal
helpers support this:

- `ReplayCursorError::variant_name() -> &'static str` — maps each error variant
  to a stable string (`"InvalidHeader"`, `"Empty"`, `"ForbiddenCharacter"`) for
  use as a structured log field.
- `log_replay_cursor_extraction_error(error, message)` — emits
  `tracing::error!` with fields `header_name = LAST_EVENT_ID_HEADER`,
  `error_variant = error.variant_name()`, and the provided static message.

Both helpers are `pub(crate)` and live in `src/sse/replay_cursor.rs`.

The tracing design follows the same principle as the pagination module:
operator-relevant failures are logged at the framework adapter boundary.
Caller-controlled failures, such as malformed client headers, are still
returned as typed errors for the caller to map to HTTP responses; the Actix
adapter additionally logs them to aid operators diagnosing misbehaving clients.

### Error mapping

The `map_replay_cursor_error` function maps each `ReplayCursorError` variant to
an `ErrorCode::InvalidRequest` error with a descriptive message:

- `ReplayCursorError::Empty` → "last-event-id must not be empty"
- `ReplayCursorError::ForbiddenCharacter` → "last-event-id must not contain
  carriage return, line feed, or null"
- `ReplayCursorError::InvalidHeader` → "last-event-id header is malformed"

These messages are suitable for client-facing error responses and follow the
same pattern as `map_idempotency_key_error`.

## Build tooling

Netsuke is the repository build driver. It compiles the root `Netsukefile` into
a Ninja build graph, then Ninja runs the selected action. There is no
`Makefile`; run every gate through `netsuke build <action>`. Install Ninja and
the published `netsuke-build` v0.1.0-beta4 release before running repository
gates. The release builds with the dated nightly pinned by its own toolchain
file:

```bash
sudo dnf install ninja-build
rustup toolchain install nightly-2026-08-23
cargo +nightly-2026-08-23 install --locked netsuke-build --version =0.1.0-beta4
```

List the available actions and their descriptions with:

```bash
netsuke help targets
```

Running `netsuke build` with no action runs `all`, which executes `spelling`,
`test-workflow-contracts`, `check-fmt`, `lint`, and `test` in that order. It
uses `dependency_order: serial`, so a failing gate stops the later ones.

The manifest targets the Netsuke v0.1.0-beta4 recipe contract, documented in
the vendored [Netsuke user's guide](netsuke-users-guide.md) and
[v0.1.0 migration guide](netsuke-v0-1-0-migration-guide.md). Write shell
dollars normally: Netsuke escapes them for Ninja, so a recipe that writes
`$$PATH` would hand the shell its own process identifier. Multi-step actions
use YAML command lists, which run in one shell and stop at the first failing
entry.

Each action prepends `$HOME/.cargo/bin`, `$HOME/.bun/bin`, and
`$HOME/.local/bin` while preserving the caller-provided `PATH`. This keeps
actions working when hook environments omit common user install directories.
Cargo-based actions honour `CARGO`, `RUST_FLAGS`, `RUSTDOC_FLAGS`, and
`BUILD_JOBS`; the remaining tools honour `MDLINT`, `MDTABLEFIX`, `NIXIE`,
`WHITAKER`, and `UV`.

Override Cargo by exporting `CARGO` before invoking Netsuke:

```bash
CARGO=/path/to/cargo netsuke build test
```

Install `cargo-nextest` to `$HOME/.cargo/bin` to enable `netsuke build test` to
use nextest:

```bash
cargo install cargo-nextest
```

The `test` action chooses its runner when the recipe runs, after the `PATH`
prepend, so it finds nextest even when Netsuke was started from a reduced
`PATH`. If `cargo-nextest` is absent, it falls back to
`cargo test --all-targets --all-features`. Neither runner executes doctests, so
the action then runs `cargo test --doc --workspace --all-features` in both
cases. The `lint` action always runs `whitaker` after rustdoc and Clippy; the
Whitaker suite is a required gate.

Markdown linting uses `MDLINT`, resolved through the same scoped prepend. In
the standard development environment, `markdownlint-cli2` resolves from
`$HOME/.bun/bin`:

```bash
netsuke build markdownlint
```

Two test targets hold the manifest and CI to this contract.
`tests/netsuke_actions.rs` runs the real `netsuke` against the committed
`Netsukefile` in a private sandbox with fake tools. The sandbox's `HOME` and
`PATH` exclude the host's own tools. The tests assert each gate's commands,
order, and flags, the serial and fail-fast behaviour of `all` and `lint`, both
test-runner branches, the `PATH` prepend, and the `CARGO` and `RUST_FLAGS`
overrides. `tests/netsuke_workflow_contract.rs` requires every workflow job
that runs a gate or the test suite to install `netsuke-build` 0.1.0-beta4 and
Ninja first. It also forbids `make` calls, and requires CI to run every gate
through an action the manifest declares. The workflow-contract target only
parses files and runs no executables. The action tests resolve `netsuke` and
`ninja` from the host `PATH` before narrowing it, and hand Ninja to Netsuke
through `NETSUKE_NINJA`. So `cargo test` needs both on `PATH`, and fails rather
than skips without them.

Ninja writes `.ninja_log`, and Netsuke keeps its generated serial-ordering
state under `.netsuke/`; both are ignored by Git. The manifest does not install
missing tools or create shims.

## The build standard

Development, test, lint and typecheck builds use the parallel `rustc` frontend
(`-Zthreads=8`) and, on Linux, the `mold` linker (`-Clink-arg=-fuse-ld=mold`).
These are defaults in `.cargo/config.toml`, which Cargo discovers on its own,
so a bare `cargo build` gets them. `mold` ships for Linux only, so the linker
flag lives in a Linux-only table and macOS and Windows keep their platform
linker. Cargo selects one `rustflags` source rather than merging them, so every
source repeats the same flags apart from the linker. The decision is recorded in
[ADR 0006](adrs/adr-0006-rust-build-standard.md).

An assigned `RUSTFLAGS` replaces the configuration's flags, so the Netsukefile
recipes that set it compose the standard's flags onto any inherited value (CI's
`setup-rust` exports one). Two builds are deliberately excluded: coverage
assigns `RUSTFLAGS: -D warnings` without the fast flags, because a measurement
should not depend on them, and `release` assigns the inherited value alone, so
a shipped artefact keeps the platform linker.

### Development build footprint

The development profile sets `incremental = false` and `debug = 0` in
`.cargo/config.toml`, so a dev build keeps no incremental state or debug
information. To debug, set `CARGO_PROFILE_DEV_DEBUG=true`; to rebuild
incrementally, set `CARGO_INCREMENTAL=1` (unset any `sccache` wrapper first,
which refuses incremental builds). The same file sets
`[unstable] embed-metadata = false`, the native Cargo key for nightlies from
2026-07-31, so Cargo coordinates metadata output itself. No raw rustc metadata
flags are used, and the pin is not moved to suit the key.
`tests/build_footprint_contract.rs` holds all three settings and reads them by
parsing the repository's own configuration. It starts no Cargo process.

### Whitaker isolation

The `lint` recipe runs Whitaker from the checkout's parent directory with an
absolute `--manifest-path` and the literal `RUSTFLAGS="-D warnings"`, after
clearing an inherited `CARGO_ENCODED_RUSTFLAGS`, which Cargo ranks above
`RUSTFLAGS`. Whitaker runs under the toolchain its installer manages, so it
takes none of the development standard (no parallel frontend, no `mold` linker
flag and no inherited `RUSTFLAGS`), and because it starts outside the checkout,
Cargo never discovers `.cargo/config.toml` and its Cranelift profile, which
that toolchain does not carry. `tests/netsuke_actions.rs` proves the working
directory, the manifest path and the literal flags against the real manifest.

### Cranelift

Cranelift is the development-profile codegen backend. The full suite was
measured under it on the pinned `nightly-2026-10-30` on 2026-10-31: all 251
nextest tests and 26 doctests pass (`netsuke build test`). Coverage selects
LLVM explicitly (`CARGO_PROFILE_DEV_CODEGEN_BACKEND=llvm`), because
instrumentation needs it, and release builds use the release profile, which
Cranelift does not touch. Re-measure the whole suite on the next toolchain
bump; if it fails, record the failing tests here as an exception and remove the
backend from `.cargo/config.toml`.

## Quality gates

All changes to the SSE module (and the broader crate) must pass the following
gates before commit:

### Formatting

```bash
netsuke build check-fmt
```

Validates that all Rust code is formatted per `rustfmt` conventions. Run
`netsuke build fmt` to apply formatting fixes.

### Lint

```bash
netsuke build lint
```

Executes Clippy with strict warnings (`-D warnings`) and the repository's
Whitaker custom lint suite. The Whitaker driver build can take up to 7 minutes
on first run; subsequent runs are fast.

Key enforced lints:

- `unwrap_used`, `expect_used` — no panics in production code
- `indexing_slicing` — bounds-checked access only
- `cognitive_complexity` — functions must be simple and clear
- `missing_const_for_fn` — mark eligible functions as `const`
- `missing_docs` — all public items require documentation

### Tests

```bash
netsuke build test
```

Runs `cargo nextest run` (if available, otherwise `cargo test`) plus
`cargo test --doc` for doctest coverage. All tests must pass. The SSE module
tests cover:

- Happy path validation for various identifier formats (ASCII, Unicode, emoji,
  numeric, UUID-formatted)
- Rejection of empty identifiers
- Rejection of identifiers containing forbidden characters (CR, LF, NULL) at
  start, middle, and end positions
- Preservation of leading and trailing whitespace
- Header extraction with missing, empty, duplicate, and non-UTF-8 headers
- Event frame rendering with optional `id:` and `event:` fields
- Data and comment newline normalization, including blank lines and trailing
  newlines
- Cache-control helper behaviour, replacement semantics, and determinism
- Property tests for `EventId` forbidden-byte validation and cache-control
  idempotence
- Heartbeat policy defaults, override validation, and heartbeat frame output
- Standard `stream_reset` event output and constant alignment
- Conversion traits (`AsRef<str>`, `Display`, `From`, `TryFrom`)
- Error mapping to API error envelope

### Documentation examples

The documentation style guide requires every user-facing fenced example to be
marked and exercised. The crate enforces that requirement for `README.md` and
`docs/users-guide.md` with a shared loader and two test targets.

Each fence must sit immediately below a `tested-example` marker, for example
`<!-- tested-example: guide-render-event-frame -->`. An identifier starts with
a lowercase ASCII letter, continues with lowercase letters, digits, and single
hyphens, and does not end with a hyphen. The loader in
`tests/documentation_examples/mod.rs` rejects an unmarked fence, an
unterminated fence, a fence without a language, an empty identifier, and a
duplicate identifier, so a new example cannot silently escape the suite.

`tests/documentation_examples_tests.rs` holds the closed registry of example
identifiers and their fence languages, compiles and runs every Rust fence in an
isolated Cargo workspace under `target/documentation-examples`, and checks the
shell and plain-text fences against the behaviour they claim. The generated
package is seeded with the repository's `Cargo.lock`, so its offline build
reuses the versions the repository already compiles rather than resolving
afresh and rejecting a crate version that crates.io has since yanked. It strips
inherited `RUSTFLAGS` because the published fences are illustrative snippets
rather than warning-clean production code.

`tests/documentation_examples/properties.rs` states the loader's contract over
generated documents: a document it accepts round-trips to the examples it was
rendered from, and an unmarked fence, a duplicate identifier, or an identifier
with a disallowed character is always rejected.

`tests/documentation_examples_bdd.rs` shares the same loader and cross-checks
the documented operator output against the tracing events the crate emits at
runtime. The scenario fails if a documented field name or value drifts from the
live event.

CI runs the examples as a named gate:

```bash
netsuke build test-documentation-contracts
```

`tests/netsuke_actions.rs` drives that gate, like every other action, against
the committed manifest with fake tools, and covers both the `cargo-nextest` and
`cargo test` branches. `netsuke build test` also covers the same targets
locally. When adding a fence, add its marker, extend `EXPECTED_EXAMPLES` with
its identifier and language, and add a semantic contract for any non-Rust
language.

### Testing dependencies

Test-only dependencies live under `[dev-dependencies]` in `Cargo.toml`.
Approved testing libraries are:

- `rstest` for fixtures and named parameterized examples.
- `rstest-bdd` and `rstest-bdd-macros` for downstream-facing behavioural
  scenarios where Gherkin structure clarifies the contract.
- `proptest` for compact invariants that should hold across generated input
  spaces, such as validation, parsing, normalization, and idempotence.
- `insta` for snapshot coverage of stable rendered output, such as error
  display text and OpenAPI schema JSON.
- `tracing-subscriber` (with the `env-filter` feature) provides composable
  tracing subscriber layers used in integration and behavioural tests.
- `tests/support/sse_trace_buffer.rs` provides `BufferLayer` and `LogBuffer`,
  which capture per-scenario tracing events into a `World`-owned buffer,
  removing the dependency on `tracing-test`'s internal API.

### Documentation

```bash
netsuke build markdownlint  # Lint Markdown files
netsuke build nixie         # Validate Mermaid diagrams
```

Markdown files must pass `markdownlint` and any embedded Mermaid diagrams must
pass `nixie` validation.

## File size guidance

No single code file may exceed 400 lines. The SSE module respects this
constraint by splitting responsibilities across focused files:

- `event_id.rs` — identifier validation (currently ~250 lines including tests)
- `replay_cursor.rs` — cursor type and header extraction (currently ~290 lines
  including tests)
- `frame.rs` — event/comment rendering rules and regression tests
- `cache_control.rs` — cache policy helper and regression tests

If a file grows beyond 400 lines, extract helper functions to a new module file
or split behavioural test suites to a dedicated `tests/` subdirectory.

## Testing conventions

### Unit tests

Use `rstest` for parameterized test cases where multiple inputs share the same
assertion structure:

```rust
#[rstest]
#[case("\n")]
#[case("evt\n123")]
#[case("evt-123\n")]
fn new_rejects_identifier_containing_line_feed(#[case] input: &str) {
    let result = EventId::new(input);
    assert_eq!(result, Err(EventIdValidationError::ForbiddenCharacter));
}
```

Avoid underscore-prefixed label parameters in rstest cases (triggers
`used_underscore_binding` lint).

### Property tests

Use `proptest` when a helper has a compact invariant that should hold across a
large input space, especially for parsing, validation, normalization, and
idempotent mutation. Keep property tests beside the helper they exercise, in
the same module-local `#[cfg(test)] mod tests` block as the related
example-based coverage.

Prefer `rstest` for named edge cases that document specific examples. Prefer
`proptest` when the important claim is universal, such as "all UTF-8 strings
without forbidden bytes are valid" or "applying this header policy twice leaves
one canonical header". Property tests should:

- use narrow strategies that match the invariant under test;
- keep generated collection sizes bounded so `netsuke build test` remains fast;
- use `prop_assert!` and `prop_assert_eq!` for failures that shrink clearly;
- avoid returning `Result` from the generated test body unless the property
  genuinely needs fallible setup outside the assertion path; and
- preserve handwritten regression tests for named boundary cases that are easy
  for reviewers to read.

Current SSE examples are:

- `sse::event_id::tests::arbitrary_byte_validation_matches_spec`, which checks
  `EventId::try_from(String)` over arbitrary byte vectors converted to a
  `String` with lossy UTF-8 decoding (`String::from_utf8_lossy`). This does not
  filter out invalid-UTF-8 inputs before calling `EventId::try_from(String)`.
- `sse::cache_control::tests::apply_event_stream_cache_control_is_idempotent_over_arbitrary_headers`,
  which checks cache-control canonicalization over generated header vectors.

### Behavioural tests

Use `rstest-bdd` when the scenario structure adds clarity. The SSE module keeps
detailed edge-case coverage in module-local unit tests, then uses
`tests/sse_wire_contract_bdd.rs` and `tests/features/sse_wire_contract.feature`
for downstream-style scenarios that compose the public crate-root re-exports.
Keep this split: add precise validation permutations beside the helper being
tested, and add behavioural scenarios only when they prove the published wire
contract more clearly than a direct assertion test.

### Test organization

Module-local/unit tests live in `#[cfg(test)] mod tests` blocks within the
implementation file. Integration contract tests live under `tests/` and must
import from `actix_v2a`, not private module paths. Module-level tests use `//!`
comments to describe coverage scope:

```rust
#[cfg(test)]
mod tests {
    //! Regression coverage for SSE event identifier validation.

    use super::*;

    #[test]
    fn new_accepts_simple_ascii_identifier() {
        // ...
    }
}
```

## Next steps

The initial SSE wire-helper milestone from roadmap tasks 1.1.1 through 1.1.3 is
now complete:

- validated event identifiers and replay cursors;
- deterministic frame and cache-header helpers;
- typed heartbeat policy plus canonical heartbeat and `stream_reset` helpers.

See [`roadmap.md`](roadmap.md) for the full delivery plan.

## Pagination module internals

The `src/pagination/` module provides reusable cursor pagination primitives
without binding the crate to a database, repository, or route shape.

### Pagination module layout

- `src/pagination/mod.rs` — module documentation, public re-exports, and the
  documented pagination error-mapping table.
- `src/pagination/cursor.rs` — `Cursor<Key>` encoding and decoding, direction
  handling, token validation, and tracing spans.
- `src/pagination/params.rs` — `PageParams` parsing, limit normalization, and
  shared query parameter constants.
- `src/pagination/envelope.rs` — `Paginated<T>` response envelopes and
  `PaginationLinks` link construction.

### Cursor ordering invariant

Cursor keys must implement `Serialize` and `Deserialize` with a consistent
representation that is compatible with the endpoint's total ordering. The
module encodes and decodes opaque cursor tokens, but it does not prove that a
database query, index, or repository predicate applies the same ordering on
every page. Downstream persistence logic owns that invariant.

### Limit normalization

`PageParams` normalizes page limits consistently:

- missing `limit` values use `DEFAULT_LIMIT`;
- limits greater than `MAX_LIMIT` are clamped to `MAX_LIMIT`;
- `limit=0` is rejected with `PageParamsError::InvalidLimit`.

<!-- markdownlint-disable-next-line MD024 -->
### Tracing

`Cursor::encode` and `Cursor::decode` are instrumented with `#[instrument]`
spans. `CursorError::Serialize` additionally emits a `tracing::error!` event at
the public `Cursor::encode` boundary because it indicates that the server could
not serialize its own cursor key. The caller-controlled variants
`CursorError::InvalidBase64`, `CursorError::Deserialize`,
`CursorError::TokenTooLong`, and `PageParamsError::InvalidLimit` do not emit
library error events.

### Pagination error mapping

HTTP adapters should map caller-controlled pagination failures to HTTP 400 and
`ErrorCode::InvalidRequest`:

- `CursorError::InvalidBase64`
- `CursorError::Deserialize`
- `CursorError::TokenTooLong`
- `PageParamsError::InvalidLimit`

`CursorError::Serialize` should map to HTTP 500 and `ErrorCode::InternalError`
because it is a server-side serialization failure.

### Testing patterns

Pagination tests are split by contract:

- `tests/pagination_documentation_bdd.rs` validates documented invariants for
  limits, cursor errors, and display text.
- `tests/pagination_http_bdd.rs` verifies handler-level HTTP status and
  `ErrorCode` mappings.
- `tests/pagination_tracing_tests.rs` verifies cursor span and event
  observability.
- `tests/snapshots/` stores `insta` snapshots for error `Display` outputs and
  OpenAPI schema JSON.

## Additional resources

- [ADR 0001: Shared SSE wire contract for Wildside and
  Corbusier](adrs/adr-0001-shared-sse-wire-contract-for-wildside-and-corbusier.md)
  — normative specification for the SSE module
- [ExecPlan: Implement SSE identifier and replay cursor
  helpers](execplans/1-1-1-implement-sse-identifier-and-replay-cursor-helpers.md)
  — implementation plan for task 1.1.1
- [ExecPlan: Implement SSE frame and cache-header
  helpers](execplans/1-1-2-sse-frame-and-cache-header-helpers.md) —
  implementation plan for task 1.1.2
- [Port Wildside pagination documentation hardening
  execplan](execplans/portwildsidepagination.md) — records the port scope,
  constraints, surprises, and acceptance criteria for the pagination
  documentation hardening workstream
- [AGENTS.md](../AGENTS.md) — code style, testing, and commit conventions

## Planned mutation and HTTP extensions

The [shared mutation design](shared-mutation-contract-design.md) owns the
future internal contract, permitted call sites, and composition rules for
roadmap phases 3 and 4. These interfaces are not implemented yet. Extend the
existing feature modules, keep transaction and authorization policy in
consumers, and publish a downstream-consumable conformance harness. Resolve
proposed ADR decisions before completing their dependent tasks. Existing
completed execution plans remain historical records.
