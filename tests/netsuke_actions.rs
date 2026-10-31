//! Behavioural checks for the `Netsukefile` gate actions.
//!
//! The repository has no `Makefile`: every gate is a Netsuke action, so the
//! manifest's recipes are the contract contributors and CI rely on. These tests
//! drive the real `netsuke` binary against the committed manifest, with fake
//! tools that log their arguments and selected environment, and assert the
//! observable result: which commands run, in what order, with which flags, and
//! whether a failure stops the gate.

#[path = "netsuke_actions/documentation.rs"]
mod documentation;

#[path = "netsuke_actions/gates.rs"]
mod gates;

#[path = "netsuke_actions/sandbox.rs"]
mod sandbox;

use anyhow::{Context, Result, ensure};
use gates::{CHECK_FMT, Gate, LintFailure, MARKDOWNLINT, Recipe, Runner, UvGate, lint_commands};
use rstest::rstest;
use sandbox::{ActionSandbox, ChildVar, FakeTool, ToolHome};

/// Runs `gate` and reports whether Netsuke exited successfully.
fn run(sandbox: &ActionSandbox, gate: Gate) -> Result<bool> {
    let output = sandbox.netsuke(&["build", gate.action()])?;
    Ok(output.status.success())
}

/// Asserts the logged commands equal `expected`, showing both on failure.
fn ensure_commands(sandbox: &ActionSandbox, expected: &[&str]) -> Result<()> {
    let commands = sandbox.commands()?;
    ensure!(
        commands == expected,
        "commands differ\nexpected: {expected:#?}\nactual: {commands:#?}"
    );
    Ok(())
}

/// `test` uses nextest when it is installed and `cargo test` otherwise, and
/// always runs doctests, all with warnings denied.
#[rstest]
#[case::nextest(Runner::Nextest)]
#[case::cargo_test_fallback(Runner::CargoTest)]
fn test_selects_the_available_runner(#[case] runner: Runner) -> Result<()> {
    let sandbox = ActionSandbox::with_default_tools()?;
    if runner.has_nextest() {
        sandbox.install(FakeTool::CargoNextest, ToolHome::Path)?;
    }
    ensure!(run(&sandbox, Gate::Test)?, "test action failed");
    ensure_commands(&sandbox, runner.commands())?;
    let environment = sandbox.environment()?;
    ensure!(
        environment
            .iter()
            .all(|record| record.contains("RUSTFLAGS=-D warnings")),
        "every test command must deny warnings: {environment:#?}"
    );
    Ok(())
}

/// `lint` runs rustdoc, Clippy, then Whitaker, each with warnings denied, and Whitaker alone
/// starts outside the checkout, clears an inherited encoded-flags variable, and takes only the
/// literal warning flags.
#[rstest]
fn lint_runs_rustdoc_clippy_then_whitaker_with_warnings_denied() -> Result<()> {
    let mut sandbox = ActionSandbox::with_default_tools()?;
    sandbox.set_var(ChildVar::InheritedEncodedRustflags, "-Awarnings");
    ensure!(run(&sandbox, Gate::Lint)?, "lint action failed");
    let parent = sandbox.checkout_parent()?;
    let manifest = format!("{parent}/work/Cargo.toml");
    let commands = lint_commands(&manifest);
    let expected: Vec<&str> = commands.iter().map(String::as_str).collect();
    ensure_commands(&sandbox, &expected)?;
    let environment = sandbox.environment()?;
    let expected_flags = [
        ("cargo doc", "RUSTDOCFLAGS=-D warnings"),
        ("whitaker --manifest-path", "RUSTFLAGS=-D warnings\t"),
        ("whitaker --manifest-path", "ENCODED=\t"),
    ];
    for (command, flag) in expected_flags {
        ensure!(
            environment
                .iter()
                .any(|record| record.starts_with(command) && record.contains(flag)),
            "{command} must run with {flag}: {environment:#?}"
        );
    }
    let cwd = format!("CWD={parent}");
    ensure!(
        environment
            .iter()
            .any(|record| record.starts_with("whitaker --manifest-path") && record.ends_with(&cwd)),
        "whitaker must start exactly in {parent}: {environment:#?}"
    );
    Ok(())
}

/// A failing `lint` step fails the action and stops the later steps.
#[rstest]
#[case::rustdoc_fails(LintFailure::Rustdoc)]
#[case::clippy_fails(LintFailure::Clippy)]
fn lint_stops_at_the_first_failing_step(#[case] failure: LintFailure) -> Result<()> {
    let mut sandbox = ActionSandbox::with_default_tools()?;
    sandbox.set_var(ChildVar::FailingCommand, failure.failing_command());
    ensure!(
        !run(&sandbox, Gate::Lint)?,
        "lint must fail when {failure:?} fails"
    );
    ensure_commands(&sandbox, failure.attempted())
}

/// Whitaker is a required gate, so `lint` fails when it is not installed.
#[rstest]
fn lint_fails_when_whitaker_is_missing() -> Result<()> {
    let sandbox = ActionSandbox::empty()?;
    sandbox.install(FakeTool::Cargo, ToolHome::Path)?;
    ensure!(
        !run(&sandbox, Gate::Lint)?,
        "lint must fail without whitaker"
    );
    ensure_commands(&sandbox, LintFailure::Clippy.attempted())
}

/// `check-fmt` checks Rust formatting, then Markdown tables.
#[rstest]
fn check_fmt_checks_rust_then_markdown_tables() -> Result<()> {
    let sandbox = ActionSandbox::with_default_tools()?;
    ensure!(run(&sandbox, Gate::CheckFmt)?, "check-fmt action failed");
    ensure_commands(&sandbox, &CHECK_FMT)
}

/// The `uv` gates run their pinned tool with uv's cache in the repository.
#[rstest]
#[case::spelling(UvGate::SPELLING)]
#[case::workflow_contracts(UvGate::WORKFLOW_CONTRACTS)]
fn uv_gates_run_the_pinned_tool_with_a_repository_cache(#[case] uv_gate: UvGate) -> Result<()> {
    let sandbox = ActionSandbox::with_default_tools()?;
    ensure!(run(&sandbox, uv_gate.gate)?, "{uv_gate:?} action failed");
    let commands = sandbox.commands()?;
    ensure!(
        uv_gate.is_sole_invocation(&commands),
        "{uv_gate:?} ran unexpected commands: {commands:#?}"
    );
    let environment = sandbox.environment()?;
    ensure!(
        environment
            .iter()
            .all(|record| record.contains("UV_CACHE_DIR=.uv-cache")),
        "{uv_gate:?} must keep uv's cache in the repository: {environment:#?}"
    );
    Ok(())
}

/// `all` runs spelling, contracts, `check-fmt`, `lint`, then `test`.
#[rstest]
fn all_runs_every_gate_serially_in_declared_order() -> Result<()> {
    let sandbox = ActionSandbox::with_default_tools()?;
    sandbox.install(FakeTool::CargoNextest, ToolHome::Path)?;
    ensure!(run(&sandbox, Gate::All)?, "all action failed");
    let commands = sandbox.commands()?;
    let gates: Vec<Option<Gate>> = commands.iter().map(|line| Gate::issuing(line)).collect();
    let expected = [
        Gate::Spelling,
        Gate::WorkflowContracts,
        Gate::CheckFmt,
        Gate::CheckFmt,
        Gate::Lint,
        Gate::Lint,
        Gate::Lint,
        Gate::Test,
        Gate::Test,
    ]
    .map(Some);
    ensure!(gates == expected, "gate order differs: {commands:#?}");
    Ok(())
}

/// A failing gate stops `all` before any later gate starts.
#[rstest]
fn all_stops_before_later_gates_when_one_fails() -> Result<()> {
    let mut sandbox = ActionSandbox::with_default_tools()?;
    sandbox.set_var(ChildVar::FailingCommand, "mdtablefix");
    ensure!(
        !run(&sandbox, Gate::All)?,
        "all must fail when check-fmt fails"
    );
    let commands = sandbox.commands()?;
    ensure!(
        commands.last().map(String::as_str) == CHECK_FMT.last().copied(),
        "check-fmt must be the last gate attempted: {commands:#?}"
    );
    ensure!(
        !commands
            .iter()
            .any(|line| Gate::issuing(line) == Some(Gate::Lint)),
        "lint must not start after check-fmt fails: {commands:#?}"
    );
    Ok(())
}

/// Actions find tools in `$HOME/.local/bin` even when `PATH` omits it.
#[rstest]
fn actions_prepend_user_tool_directories_to_path() -> Result<()> {
    let sandbox = ActionSandbox::empty()?;
    sandbox.install(FakeTool::Nixie, ToolHome::LocalBin)?;
    ensure!(
        run(&sandbox, Gate::Nixie)?,
        "nixie in $HOME/.local/bin must be found"
    );
    ensure_commands(&sandbox, &["nixie\t--no-sandbox"])
}

/// `CARGO` replaces the Cargo binary and `RUST_FLAGS` extends `RUSTFLAGS`.
#[rstest]
fn cargo_and_rust_flag_overrides_reach_the_recipe() -> Result<()> {
    let mut sandbox = ActionSandbox::empty()?;
    sandbox.install(FakeTool::PinnedCargo, ToolHome::Path)?;
    sandbox.set_var(ChildVar::Cargo, "pinned-cargo");
    sandbox.set_var(ChildVar::RustFlags, "--cfg sandbox");
    ensure!(run(&sandbox, Gate::Typecheck)?, "typecheck action failed");
    ensure_commands(
        &sandbox,
        &["pinned-cargo\tcheck\t--all-targets\t--all-features"],
    )?;
    let environment = sandbox.environment()?;
    ensure!(
        environment.iter().any(|record| {
            record.contains("RUSTFLAGS=-D warnings -Zthreads=8") && record.contains("--cfg sandbox")
        }),
        "RUST_FLAGS must extend the denied-warnings and standard flags: {environment:#?}"
    );
    Ok(())
}

/// The development recipes compose the standard's flags onto an inherited `RUSTFLAGS`, as CI's
/// `setup-rust` exports one; the release build keeps the platform linker and frontend.
#[rstest]
#[case::build(Gate::Build, "cargo build", true)]
#[case::typecheck(Gate::Typecheck, "cargo check", true)]
#[case::test_run(Gate::Test, "cargo test", true)]
#[case::documentation_contracts(Gate::DocumentationContracts, "cargo test", true)]
#[case::clippy(Gate::Lint, "cargo clippy", true)]
#[case::release(Gate::Release, "cargo build", false)]
fn the_standard_flags_reach_development_recipes_but_not_release(
    #[case] gate: Gate,
    #[case] command: &str,
    #[case] takes_standard: bool,
) -> Result<()> {
    let mut sandbox = ActionSandbox::with_default_tools()?;
    sandbox.set_var(ChildVar::InheritedRustflags, "-D warnings");
    ensure!(run(&sandbox, gate)?, "{gate:?} failed");
    let environment = sandbox.environment()?;
    let record = environment
        .iter()
        .find(|record| record.starts_with(command))
        .with_context(|| format!("no {command} record: {environment:#?}"))?;
    ensure!(
        record.contains("RUSTFLAGS=-D warnings"),
        "{command}: the inherited RUSTFLAGS must reach the build: {record}"
    );
    ensure!(
        record.contains("-Zthreads=8") == takes_standard,
        "{command}: the parallel frontend flag must be {takes_standard}: {record}"
    );
    ensure!(
        record.contains("-fuse-ld=mold") == (takes_standard && cfg!(target_os = "linux")),
        "{command}: the mold flag is Linux-only and development-only: {record}"
    );
    Ok(())
}

/// The build, clean, and format actions run their commands in order.
#[rstest]
#[case::build(Recipe::BUILD)]
#[case::release(Recipe::RELEASE)]
#[case::clean(Recipe::CLEAN)]
#[case::fmt(Recipe::FMT)]
fn fixed_recipes_run_their_commands(#[case] recipe: Recipe) -> Result<()> {
    let sandbox = ActionSandbox::with_default_tools()?;
    ensure!(run(&sandbox, recipe.gate)?, "{recipe:?} failed");
    ensure_commands(&sandbox, recipe.commands)
}

/// A failing `fmt` step fails the action and stops the later steps.
#[rstest]
fn fmt_stops_at_the_first_failing_step() -> Result<()> {
    let mut sandbox = ActionSandbox::with_default_tools()?;
    sandbox.set_var(ChildVar::FailingCommand, "cargo fmt");
    ensure!(
        !run(&sandbox, Gate::Fmt)?,
        "fmt must fail when rustfmt fails"
    );
    ensure_commands(&sandbox, Recipe::FMT.commands.split_at(1).0)
}

/// `BUILD_JOBS` reaches the build actions as separate Cargo arguments.
#[rstest]
#[case::build(Gate::Build, "cargo\tbuild\t-j\t2")]
#[case::release(Gate::Release, "cargo\tbuild\t--release\t-j\t2")]
fn build_jobs_reach_cargo(#[case] gate: Gate, #[case] expected: &str) -> Result<()> {
    let mut sandbox = ActionSandbox::with_default_tools()?;
    sandbox.set_var(ChildVar::BuildJobs, "-j 2");
    ensure!(run(&sandbox, gate)?, "{gate:?} failed");
    ensure_commands(&sandbox, &[expected])
}

/// `markdownlint` runs the spelling gate first, then lints every Markdown file.
#[rstest]
fn markdownlint_checks_spelling_first() -> Result<()> {
    let sandbox = ActionSandbox::with_default_tools()?;
    ensure!(run(&sandbox, Gate::Markdownlint)?, "markdownlint failed");
    let commands = sandbox.commands()?;
    let ran_in_order = commands.split_last().is_some_and(|(last, earlier)| {
        last == MARKDOWNLINT && UvGate::SPELLING.is_sole_invocation(earlier)
    });
    ensure!(
        ran_in_order,
        "spelling then markdownlint expected: {commands:#?}"
    );
    Ok(())
}

/// A spelling failure stops `markdownlint` before it lints anything.
#[rstest]
fn markdownlint_stops_when_spelling_fails() -> Result<()> {
    let mut sandbox = ActionSandbox::with_default_tools()?;
    sandbox.set_var(ChildVar::FailingCommand, "uv");
    ensure!(
        !run(&sandbox, Gate::Markdownlint)?,
        "markdownlint must fail"
    );
    let commands = sandbox.commands()?;
    ensure!(
        UvGate::SPELLING.is_sole_invocation(&commands),
        "only the spelling gate should have run: {commands:#?}"
    );
    Ok(())
}

/// `netsuke help targets` lists every gate action.
#[rstest]
fn help_targets_lists_every_gate() -> Result<()> {
    let sandbox = ActionSandbox::empty()?;
    let output = sandbox.netsuke(&["help", "targets"])?;
    ensure!(output.status.success(), "netsuke help targets failed");
    let listing = String::from_utf8_lossy(&output.stdout);
    let actions = [
        "all",
        "build",
        "release",
        "clean",
        "test",
        "lint",
        "typecheck",
        "fmt",
        "check-fmt",
        "markdownlint",
        "spelling",
        "test-documentation-contracts",
        "test-workflow-contracts",
        "nixie",
    ];
    let missing: Vec<&str> = actions
        .into_iter()
        .filter(|action| {
            !listing
                .lines()
                .any(|line| line.split_whitespace().next() == Some(*action))
        })
        .collect();
    ensure!(
        missing.is_empty(),
        "help targets omits {missing:?}:\n{listing}"
    );
    Ok(())
}
