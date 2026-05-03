# Adopt Netsuke as the Repository Build Driver

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

## Purpose / big picture

This repository currently uses GNU Make through the root `Makefile` and the
GitHub Actions workflow at `.github/workflows/ci.yml`. The goal is to pilot
Netsuke as the primary build driver for this simple Rust repository while
dogfooding Netsuke before package-manager distribution is available. After this
change, a developer can run `netsuke build check-fmt`, `netsuke build lint`,
and `netsuke build test` from the repository root and receive the same quality
gate behaviour currently exposed by `make check-fmt`, `make lint`, and
`make test`.

Success is observable in two places. Locally, the Netsuke commands pass and
produce the same effective Cargo, Markdown, and Mermaid checks as the existing
Makefile. In CI, `.github/workflows/ci.yml` installs Netsuke from
`https://github.com/leynos/netsuke/` source, installs Ninja, and invokes
Netsuke for repository gates instead of invoking `make` directly. This is a
dogfooding pilot, so the plan keeps rollback straightforward and captures any
Netsuke limitations discovered while translating the existing Makefile targets.

## Constraints

- Read and follow `AGENTS.md` before implementation. This plan was drafted from
  the repository instructions embedded in the task prompt and must be checked
  against the actual file again before execution.
- Use `docs/netsuke-users-guide.md` as the local Netsuke reference for manifest
  structure, command-line behaviour, configuration, and diagnostics.
- Install Netsuke from source in CI using the GitHub repository, not from a
  Debian package, `cargo-binstall`, or crates.io metadata.
- Keep the pilot suitable for demonstrating that Netsuke can replace `gnumake`
  in a simple repository. Do not add a bespoke task runner, shell framework, or
  wrapper script unless a documented Netsuke limitation makes it necessary.
- Preserve the existing quality gates: Rust formatting, Rustdoc and Clippy
  linting, Whitaker when available, tests including doctests, Markdown linting,
  and Mermaid validation.
- Do not change the public Rust API as part of this build-tooling migration.
- Do not introduce new Rust crate dependencies to `Cargo.toml`.
- Do not run format, lint, or test commands in parallel. The repository relies
  on shared build caching, and sequential execution is required.
- Capture long validation command output through `tee` into `/tmp`, using
  branch-specific log names.
- Keep documentation in en-GB Oxford spelling and wrap Markdown paragraphs at
  80 columns.
- Do not remove the `Makefile` until Netsuke has passed locally and in CI, and
  until the rollback story is accepted. During the pilot, retaining the
  Makefile as a compatibility shim is allowed.

If satisfying the objective requires violating a constraint, stop immediately,
record the conflict in `Decision Log`, and ask for direction.

## Tolerances (exception triggers)

- Scope: if implementation requires changes to more than five repository files
  or more than 250 net lines outside generated lock/build artefacts, stop and
  escalate.
- Interface: if any Rust public API signature must change, stop and escalate.
- Dependencies: if any new project dependency must be added to `Cargo.toml`,
  stop and escalate.
- CI: if replacing the coverage step requires removing CodeScene upload or
  changing coverage format from `lcov`, stop and present options.
- Tooling: if Netsuke cannot express an existing Makefile target without a
  wrapper script longer than 40 lines, stop and document the limitation.
- Iterations: if a gate still fails after two focused fixes, stop and document
  the failing command, log path, and options.
- Ambiguity: if there are multiple credible interpretations of "alternative to
  gnumake" that materially affect whether the Makefile remains, stop and ask.

## Risks

- Risk: Netsuke may not support Make-style conditional execution cleanly for
  the `test` target's "use nextest if installed, otherwise cargo test" logic.
  Severity: medium. Likelihood: medium. Mitigation: encode the conditional in a
  small `script:` action and keep it directly equivalent to the current
  Makefile recipe; escalate if this becomes a larger shell wrapper.
- Risk: GitHub's Ubuntu runner may not have Ninja installed. Severity: medium.
  Likelihood: medium. Mitigation: add an explicit `apt-get install ninja-build`
  CI step before invoking Netsuke.
- Risk: installing Netsuke from GitHub source with Cargo may fail if the
  upstream lockfile or Rust toolchain requirements drift. Severity: medium.
  Likelihood: low. Mitigation: pin the install to a reviewed Netsuke revision
  for the pilot, document the revision, and update it deliberately when testing
  newer Netsuke.
- Risk: The shared coverage action in `.github/workflows/ci.yml` may hide an
  internal `make` call. Severity: medium. Likelihood: unknown. Mitigation:
  inspect its documented inputs or source before finalizing CI; if it invokes
  Make, either configure it to use the Netsuke test command or replace it with
  an explicit coverage command that still uploads `lcov.info`.
- Risk: Markdown formatting tools may not be installed in every local
  environment. Severity: low. Likelihood: medium. Mitigation: preserve the
  current Makefile's `PATH` discovery behaviour in the Netsuke manifest and
  keep failure messages from the underlying tools visible.

## Progress

- [x] 2026-05-02: Create the initial draft plan on branch `adopt-netsuke`.
- [x] 2026-05-02: Inspect the current `Makefile`, `.github/workflows/ci.yml`,
  `docs/developers-guide.md`, and `docs/netsuke-users-guide.md`.
- [x] 2026-05-02: Confirm upstream Netsuke source metadata from
  `https://github.com/leynos/netsuke/`; the package name is `netsuke`, current
  `HEAD` during drafting was `2fe314a58d7311758640b3daa086c401d79838cf`, and
  the crate declares Rust `1.89.0`.
- [x] 2026-05-02: Receive explicit approval to proceed with implementation.
- [x] 2026-05-02: Re-check upstream Netsuke `HEAD`; it remains
  `2fe314a58d7311758640b3daa086c401d79838cf`.
- [x] 2026-05-02: Add a repository `Netsukefile` that reproduces current
  Makefile targets and validates with `netsuke manifest -`.
- [x] 2026-05-02: Update developer documentation to describe Netsuke-first
  usage and Makefile compatibility.
- [x] 2026-05-02: Update CI to install Netsuke from GitHub source, install
  Ninja, validate the manifest, and run Netsuke gates for format, Markdown, and
  lint.
- [x] 2026-05-02: Validate formatting, linting, tests, Markdown linting, and
  Mermaid validation through Netsuke.
- [x] 2026-05-02: Validate Makefile compatibility for `check-fmt`, `lint`,
  `test`, `markdownlint`, and `nixie`; each target delegates to Netsuke.
- [!] 2026-05-02: `make fmt` reached Markdown formatting and failed on
  line-length issues in `docs/netsuke-users-guide.md`, which is outside the
  implementation file budget in this plan. Implementation is paused for scope
  approval before modifying a sixth file.
- [x] 2026-05-02: Receive approval to exceed the five-file tolerance and fix
  Markdown formatting issues in `docs/netsuke-users-guide.md`.
- [x] 2026-05-02: Run `make fmt` successfully through Netsuke after wrapping
  long Markdown lines.
- [x] 2026-05-02: Rerun final gates: `netsuke manifest -`, `make check-fmt`,
  `make markdownlint`, `make nixie`, `make lint`, and `make test`.
- [x] 2026-05-02: Prepare the gated implementation change for commit.
- [x] 2026-05-02: Inspect
  `leynos/agent-helper-scripts/hooks/post-turn-quality-stop-hook.py` and
  document the external hook changes needed for Netsuke-native quality gates.
- [x] 2026-05-02: Address CI runtime failure `markdownlint-cli2: not found` by
  installing Bun and `markdownlint-cli2` via GitHub Actions.
- [x] 2026-05-03: Pin `oven-sh/setup-bun` and `markdownlint-cli2` in CI to
  immutable versions for reproducible installs.
- [x] 2026-05-03: Fix `typecheck` action to pass `RUSTFLAGS` in-process with
  the `cargo check` command so Netsuke enforces `-D warnings` consistently.

## Surprises & Discoveries

- 2026-05-02: The current CI only calls `make` directly for `check-fmt` and
  `lint`. Tests and coverage are delegated to
  `leynos/shared-actions/.github/actions/generate-coverage`, so implementation
  must check whether that shared action invokes `make` internally before
  claiming CI is fully Make-free.
- 2026-05-02: The Makefile already contains compatibility work for reduced
  `PATH` environments by prepending `$HOME/.cargo/bin`, `$HOME/.bun/bin`, and
  `$HOME/.local/bin`. The Netsuke manifest must preserve this behaviour.
- 2026-05-02: Netsuke requires a `targets:` key even for an action-only
  repository manifest. The implemented manifest uses `targets: []`.
- 2026-05-02: Netsuke `actions` are compiled as Ninja build edges, and an
  action can use `sources:` to sequence other actions. The `all` action uses
  this to run `check-fmt`, `lint`, and `test` before its own no-op command.
- 2026-05-02: Shell variables in Netsuke scripts must be written with doubled
  dollar signs, such as `$$PATH`, because the generated Ninja file treats `$`
  as an escape marker.
- 2026-05-02: Netsuke `script:` actions still produced Ninja `$` escaping
  failures for this manifest. Switching to `command:` actions that invoke
  `sh -e -c` preserved the shell behaviour and passed Ninja execution.
- 2026-05-02: The shared `generate-coverage` action does not call Make
  directly. It runs its own Rust coverage scripts through `cargo llvm-cov`, so
  the CI migration can keep CodeScene coverage upload intact while replacing
  direct Make invocations with Netsuke.
- 2026-05-02: `netsuke build markdownlint` in CI failed because
  `markdownlint-cli2` was missing from PATH; that gap is now addressed by
  Bun-based install steps before Netsuke invocations.
- 2026-05-02: `make fmt` now invokes Netsuke and runs, but Markdown formatting
  fails because `docs/netsuke-users-guide.md` contains long lines that
  `markdownlint --fix` reports and cannot automatically repair.
- 2026-05-02: The external post-turn quality hook is Make-specific today. It
  stores `make_targets_requested`, parses `make -qp`, runs grouped
  `make --no-print-directory ...` commands, and reports "Requested make
  targets" in its block output.
- 2026-05-03: Netsuke `typecheck` was assigning `RUSTFLAGS` to a shell variable
  without exporting it; the fixed action now invokes `cargo check` with the
  environment assignment in-line, restoring the Makefile semantics and preventing
  silent warning-only pass-through.

## Decision Log

- Decision: Treat this as a Netsuke-first pilot rather than deleting Makefile
  support immediately. Rationale: The user asked for adoption "as an
  alternative to `gnumake`" and identified this as a dogfooding pilot. Keeping
  Makefile compatibility during the first Netsuke migration reduces rollback
  risk while still proving that Netsuke can drive the repository gates.
- Decision: Install Netsuke in CI with Cargo from GitHub source and pin the
  pilot to a reviewed revision. Rationale: The user explicitly rejected Debian
  packages and `cargo-binstall` metadata until the pilot has demonstrated
  replacement value. Pinning avoids unrelated upstream Netsuke movement
  breaking this repository's CI.
- Decision: Model repository gates as Netsuke `actions`, not file-producing
  `targets`. Rationale: The current Makefile gates are phony commands that
  validate the working tree rather than producing durable artefacts. Netsuke's
  `actions:` section maps directly to that behaviour.
- Decision: Keep the `Makefile` as a compatibility shim that delegates to
  Netsuke. Rationale: This preserves existing local commands and hooks while
  ensuring Netsuke is the actual build driver during the pilot.
- Decision: Keep the existing shared coverage action in CI.
  Rationale: The action does not invoke Make and still produces `lcov.info` for
  the existing CodeScene upload step. Replacing it would add coverage risk
  outside the Netsuke adoption objective.
- Decision: Use `command:` actions with explicit `sh -e -c` invocations instead
  of Netsuke `script:` actions for this pilot. Rationale: The current Netsuke
  source rejects the generated Ninja file when script bodies contain shell
  variables. `command:` actions accept doubled dollar signs and keep the
  repository gate behaviour observable.
- Decision: Pause before editing `docs/netsuke-users-guide.md`.
  Rationale: Fixing `make fmt` requires touching a sixth file, which breaches
  this plan's file-count tolerance. The implementation restored unrelated
  formatter-only changes and now needs explicit approval to exceed scope.
- Decision: Continue after explicit scope approval.
  Rationale: The user approved continuing after the `make fmt` blocker was
  reported, so `docs/netsuke-users-guide.md` can be updated narrowly to satisfy
  Markdown formatting.
- Decision: Document agent-helper-scripts changes here instead of editing the
  external repository in this branch. Rationale: The requested script lives in
  `leynos/agent-helper-scripts`, while this pull request belongs to
  `leynos/actix-v2a`. Keeping the requirement in this ExecPlan lets the
  follow-up be implemented in the owning repository.
- Decision: Install `markdownlint-cli2` in CI through Bun with `oven-sh/setup-bun`
  and `bun install -g markdownlint-cli2`. Rationale: the Netsuke Markdown lint
  step depends on that binary and is not guaranteed present on GitHub runners.
- Decision: Pin the CI Bun setup action and markdownlint-cli2 package version to
  fixed revisions (`oven-sh/setup-bun@0c5077e51419868618aeaa5fe8019c62421857d6`
  and
  `markdownlint-cli2@0.22.1`). Rationale: reproducibility should be explicit for
  dogfooding gates.
- Decision: Keep Netsuke `typecheck` warning policy explicit by passing
  `RUSTFLAGS` directly in the `cargo check` command line. Rationale: separate
  shell variable assignment without export is equivalent to a no-op for child
  processes and can silently miss warnings that should fail CI.

## Outcomes & Retrospective

The repository now has a root `Netsukefile`, a Makefile compatibility shim,
Netsuke-first developer documentation, and CI steps that install Netsuke from
the pinned GitHub source revision before running Netsuke gates. The Netsuke
manifest, format check, lint, test, Markdown lint, Mermaid validation, and
Makefile compatibility gates passed.
CI now installs Bun and `markdownlint-cli2` explicitly so the Markdown gate no
longer fails with `markdownlint-cli2: not found` during Netsuke execution.

After approval to exceed the initial file-count tolerance,
`docs/netsuke-users-guide.md` was wrapped narrowly, `make fmt` passed, and the
final Netsuke-driven Makefile gates passed. The migration is complete from the
repository's perspective; CI must still prove the source install path on GitHub
Actions.

## Required agent-helper-scripts hook changes

The stop hook at
`https://github.com/leynos/agent-helper-scripts/blob/main/hooks/post-turn-quality-stop-hook.py`
 must become build-driver aware before repositories can retire Makefile shims.
The current script assumes Make at the data-model, discovery, execution, and
reporting layers:

- `CATS_TO_TARGETS` maps change categories to target names, but the surrounding
  state names those values as Make targets.
- `HookState` stores `make_targets_requested`, `make_targets_run`, and
  `make_targets_skipped`.
- `get_make_targets()` shells out to `make -qp --no-print-directory` and parses
  Make's database output.
- `run_make()` invokes `make --no-print-directory <targets...>`.
- `evaluate_changes()` skips requested targets that are absent from the
  Makefile.
- `format_reason()` reports "Requested make targets", "Targets run", and
  "Targets skipped (missing)".

The follow-up in `agent-helper-scripts` should introduce a small build-driver
abstraction rather than special-casing Netsuke throughout the hook. A minimal
shape is enough:

```python
@dataclass(frozen=True)
class BuildDriver:
    name: str
    executable: str
    manifest: str

    def list_targets(self, repo: Path) -> set[str]: ...
    def run_targets(
        self,
        repo: Path,
        kind: str,
        targets: list[str],
        max_out: int,
    ) -> dict[str, Any]: ...
```

Discovery should prefer Netsuke when a root `Netsukefile` exists and `netsuke`
is on `PATH`. It should fall back to Make when a `Makefile` exists or when
Netsuke is absent. This preserves current behaviour for repositories that have
not adopted Netsuke while allowing this repository to remove the compatibility
shim later.

Target enumeration for Netsuke should not parse arbitrary human output. Use one
of these approaches, in order of preference:

1. Add a Netsuke machine-readable target listing command upstream and call it
   from the hook once available.
2. Until that exists, run `netsuke manifest -` and parse generated Ninja
   `build <target>:` lines for non-implicit targets. This is less ideal, but it
   is deterministic enough for a stop-hook bridge.
3. As a temporary fallback, assume the standard target names from
   `CATS_TO_TARGETS` exist when `Netsukefile` is present, run them, and let
   Netsuke report an unknown-target failure. This is noisier but still blocks
   correctly.

Execution should run one Netsuke process per target group, matching the current
code and Markdown grouping:

```python
["netsuke", "build", *targets]
```

The hook must continue to split code and Markdown checks so Markdown-only
changes run only `markdownlint`, and Rust changes run `check-fmt` and `lint`.
If future repositories add TypeScript/Python Netsuke actions, the existing
`python_ts` mapping can continue to request `check-fmt`, `lint`, and
`typecheck`.

The hook's state and output should be renamed from Make-specific wording to
driver-neutral wording:

- `make_targets_requested` -> `targets_requested`
- `make_targets_run` -> `targets_run`
- `make_targets_skipped` -> `targets_skipped`
- "Requested make targets" -> "Requested build targets"
- failed command strings should show either `netsuke build ...` or `make ...`
  exactly as executed.

The configuration surface should gain explicit overrides for gradual rollout:

- `POST_TURN_BUILD_DRIVER=auto|netsuke|make`, defaulting to `auto`.
- `POST_TURN_NETSUKE_BIN=/path/to/netsuke`, defaulting to `netsuke`.
- `POST_TURN_MAKE_BIN=/path/to/make`, defaulting to `make`.

When `POST_TURN_BUILD_DRIVER=netsuke`, the hook should block with a clear error
if `Netsukefile` is missing or `netsuke` is unavailable. When the value is
`auto`, missing Netsuke should fall back to Make if Make is available. When no
supported driver is available and checks are required, the hook should block
with an actionable message.

The hook tests in `agent-helper-scripts` should cover:

- Netsuke is selected when both `Netsukefile` and `Makefile` exist.
- Make is selected when only `Makefile` exists.
- `POST_TURN_BUILD_DRIVER=make` forces Make even when `Netsukefile` exists.
- `POST_TURN_BUILD_DRIVER=netsuke` blocks if Netsuke is missing.
- Markdown-only changes invoke `netsuke build markdownlint`.
- Rust changes invoke `netsuke build check-fmt lint`.
- Python/TypeScript changes invoke `netsuke build check-fmt lint typecheck`.
- Failure output uses driver-neutral labels and includes the exact failed
  command.
- `POST_TURN_COMPUSH=1` behaviour is unchanged after successful Netsuke checks.

Acceptance for the hook change is that this repository can delete the Makefile
compatibility shim and still have the stop hook run the same quality gates
through `netsuke build ...`.

## Repository orientation

The current build-tooling surface is small:

- `Makefile` defines `check-fmt`, `lint`, `test`, `fmt`, `markdownlint`,
  `nixie`, `build`, `release`, `clean`, `typecheck`, `all`, and `help`.
- `.github/workflows/ci.yml` runs on pull requests and manual dispatch. It
  checks out the repository, sets up Rust, installs Bun tools, installs Netsuke,
  validates the `Netsukefile` manifest, then runs `netsuke build` gates for
  format, Markdown linting, and linting, followed by shared coverage and
  CodeScene upload steps.
- `docs/developers-guide.md` documents Makefile-based build tooling and must
  be updated so developers know that Netsuke is the preferred driver during the
  pilot.
- `docs/netsuke-users-guide.md` documents the manifest format. Important
  concepts for this migration are `actions:` for phony tasks, `command:` blocks
  for multi-line shell execution, `defaults:` for what runs when `netsuke` is
  invoked without targets, and `netsuke manifest` for inspecting the generated
  Ninja file without executing it.

## Implementation plan

First, create `Netsukefile` in the repository root. Use
`netsuke_version: "1.0.0"` and define shared variables for the tool paths and
flags currently centralised in the Makefile:

```yaml
vars:
  prepend_path: >
    {{ env('HOME') }}/.cargo/bin:{{ env('HOME') }}/.bun/bin:
    {{ env('HOME') }}/.local/bin
  cargo: "{{ env('CARGO', 'cargo') }}"
  rust_flags: "-D warnings"
  rustdoc_flags: "-D warnings"
  cargo_flags: "--all-targets --all-features"
```

Use `actions:` for the repository gates. Each action is a `command:` action that
invokes `sh -e -c` and preserves the current behaviour:

- `check-fmt` runs `netsuke build check-fmt`.
- `lint` runs `netsuke build lint`.
- `test` runs `netsuke build test` with `nextest` detection and fallback logic.
- `fmt` runs `netsuke build fmt`.
- `markdownlint` runs `netsuke build markdownlint`.
- `nixie` runs `netsuke build nixie`.
- `typecheck` runs `netsuke build typecheck`.
- `clean` runs `netsuke build clean`.
- `all` runs `netsuke build all`, where `defaults:` is defined to keep the same
  behaviour as the Makefile's `all`.

Set `defaults:` to `["all"]` because the implementation already confirms
`netsuke` without arguments should execute the full gate suite for this pilot.

Second, keep or adapt `Makefile` as a compatibility shim. The preferred pilot
shape is for `make check-fmt` and related targets to delegate to
`netsuke build check-fmt` once Netsuke is available. If this creates a
bootstrap problem for environments that have Make but not Netsuke, leave the
Makefile recipes intact and document Netsuke as the primary path instead. Do
not delete the Makefile in the first implementation unless explicitly approved.

Third, update `.github/workflows/ci.yml`. After the Rust setup step, install
Ninja and install Netsuke from source:

```yaml
- name: Install Ninja
  run: sudo apt-get update && sudo apt-get install -y ninja-build
- name: Install Netsuke
  run: >-
    cargo install --git https://github.com/leynos/netsuke.git
    --rev 2fe314a58d7311758640b3daa086c401d79838cf
    netsuke --locked
- name: Show Netsuke version
  run: netsuke --version
```

Replace direct `make check-fmt` and `make lint` invocations with
`netsuke build check-fmt` and `netsuke build lint`. Add a Netsuke-driven test
step unless the coverage action is confirmed to run tests without Make and with
equivalent coverage. If the shared coverage action invokes `make`, replace or
configure it so CI remains Netsuke-first while still producing `lcov.info` for
the existing CodeScene upload step.

Fourth, update documentation. In `docs/developers-guide.md`, change the build
tooling section to describe Netsuke as the preferred driver, include the source
install command for the pilot, keep a short compatibility note for Makefile
users if the Makefile remains, and update gate examples from `make ...` to
`netsuke build ...`. Also update any roadmap or contributor references that
would otherwise instruct contributors to use Make for the same gates. Do not
bulk-rewrite historical ExecPlans.

Fifth, validate the generated Ninja manifest before running expensive gates:

```bash
set -o pipefail && netsuke manifest - 2>&1 | tee /tmp/manifest-actix-v2a-adopt-netsuke.out
```

The command should exit successfully and print a Ninja file containing build
edges for the declared actions. If this fails, fix the `Netsukefile` before
running any gate.

## Validation plan

Run validation sequentially and capture logs in `/tmp`:

```bash
set -o pipefail && netsuke build check-fmt 2>&1 | tee /tmp/check-fmt-actix-v2a-adopt-netsuke.out
set -o pipefail && netsuke build lint 2>&1 | tee /tmp/lint-actix-v2a-adopt-netsuke.out
set -o pipefail && netsuke build test 2>&1 | tee /tmp/test-actix-v2a-adopt-netsuke.out
set -o pipefail && netsuke build markdownlint 2>&1 | tee /tmp/markdownlint-actix-v2a-adopt-netsuke.out
set -o pipefail && netsuke build nixie 2>&1 | tee /tmp/nixie-actix-v2a-adopt-netsuke.out
```

For documentation-only commits, at minimum run:

```bash
set -o pipefail && make markdownlint 2>&1 | tee /tmp/markdownlint-actix-v2a-adopt-netsuke-plan.out
set -o pipefail && make nixie 2>&1 | tee /tmp/nixie-actix-v2a-adopt-netsuke-plan.out
```

For the final implementation commit, also run the legacy Makefile gates if the
Makefile remains as a compatibility path:

```bash
set -o pipefail && make check-fmt 2>&1 | tee /tmp/make-check-fmt-actix-v2a-adopt-netsuke.out
set -o pipefail && make lint 2>&1 | tee /tmp/make-lint-actix-v2a-adopt-netsuke.out
set -o pipefail && make test 2>&1 | tee /tmp/make-test-actix-v2a-adopt-netsuke.out
```

Expected success:

- `netsuke manifest -` exits with status 0.
- `netsuke build check-fmt` reports no Rust formatting drift.
- `netsuke build lint` builds Rustdoc without warnings, runs Clippy with
  warnings denied, and either runs Whitaker or reports the existing "whitaker
  not found" skip message.
- `netsuke build test` runs the full workspace test suite and doctests.
- `netsuke build markdownlint` and `netsuke build nixie` pass after
  documentation updates.
- GitHub Actions shows the workflow installing Netsuke from source and running
  Netsuke commands for the repository gates.

## Rollback plan

If Netsuke cannot drive the gates within the tolerances above, leave the
existing Makefile and CI Make calls intact, remove any incomplete `Netsukefile`
changes, and record the Netsuke limitation in `Surprises & Discoveries` and
`Decision Log`. If only CI installation is unstable, keep the local Netsuke
manifest and continue using Make in CI until the Netsuke revision or install
method is fixed. Do not remove coverage upload unless explicitly approved.

## Approval gate

This plan is in draft. Do not implement the `Netsukefile`, CI workflow, or
developer documentation changes until the plan is explicitly approved or
revised by the user.
