//! Behavioural checks for the `Netsukefile` gate actions.
//!
//! The repository has no `Makefile`: every gate is a Netsuke action, so the
//! manifest's recipes are the contract contributors and CI rely on. These tests
//! drive the real `netsuke` binary against the committed manifest, with fake
//! tools that log their arguments and selected environment, and assert the
//! observable result: which commands run, in what order, with which flags, and
//! whether a failure stops the gate.

#[path = "netsuke_actions/gates.rs"]
mod gates;
#[path = "netsuke_actions/sandbox.rs"]
mod sandbox;

use anyhow::{Result, ensure};
use gates::{CHECK_FMT, Gate, LINT, LintFailure, Runner, UvGate};
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

/// `lint` runs rustdoc, Clippy, then Whitaker, each with warnings denied.
#[rstest]
fn lint_runs_rustdoc_clippy_then_whitaker_with_warnings_denied() -> Result<()> {
    let sandbox = ActionSandbox::with_default_tools()?;
    ensure!(run(&sandbox, Gate::Lint)?, "lint action failed");
    ensure_commands(&sandbox, &LINT)?;
    let environment = sandbox.environment()?;
    let expected_flags = [
        ("cargo doc", "RUSTDOCFLAGS=-D warnings"),
        ("whitaker --all", "RUSTFLAGS=-D warnings"),
    ];
    for (command, flag) in expected_flags {
        ensure!(
            environment
                .iter()
                .any(|record| record.starts_with(command) && record.contains(flag)),
            "{command} must run with {flag}: {environment:#?}"
        );
    }
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
        environment
            .iter()
            .any(|record| record.contains("RUSTFLAGS=-D warnings --cfg sandbox")),
        "RUST_FLAGS must extend the denied-warnings flags: {environment:#?}"
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
