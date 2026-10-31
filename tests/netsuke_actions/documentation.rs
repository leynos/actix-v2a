//! Checks the `test-documentation-contracts` gate against the fake tools.
//!
//! CI reports this gate separately so a broken guide fails with a
//! documentation-specific signal. These tests pin both branches of its runner
//! selection, the caller's job count reaching Cargo, and a failing example
//! stopping the gate with nothing later attempted.

use anyhow::{Result, ensure};
use rstest::rstest;

use super::{
    gates::{Gate, Runner},
    run,
    sandbox::{ActionSandbox, ChildVar, FakeTool, ToolHome},
};

/// Asserts the logged commands equal `expected`, showing both on failure.
fn ensure_commands(sandbox: &ActionSandbox, expected: &[String]) -> Result<()> {
    let commands = sandbox.commands()?;
    ensure!(
        commands == expected,
        "commands differ\nexpected: {expected:#?}\nactual: {commands:#?}"
    );
    Ok(())
}

/// Runs the gate with the case's runner, installing the fake nextest when the
/// case expects it to be found.
fn documentation_sandbox(runner: Runner) -> Result<ActionSandbox> {
    let sandbox = ActionSandbox::with_default_tools()?;
    if runner.has_nextest() {
        sandbox.install(FakeTool::CargoNextest, ToolHome::Path)?;
    }
    Ok(sandbox)
}

/// `test-documentation-contracts` runs only the documentation targets, with
/// the available runner, the caller's job count, and warnings denied.
#[rstest]
#[case::nextest(Runner::Nextest)]
#[case::cargo_test_fallback(Runner::CargoTest)]
fn documentation_contracts_selects_the_available_runner(#[case] runner: Runner) -> Result<()> {
    let mut sandbox = documentation_sandbox(runner)?;
    sandbox.set_var(ChildVar::BuildJobs, "-j 2");
    ensure!(
        run(&sandbox, Gate::DocumentationContracts)?,
        "test-documentation-contracts action failed"
    );
    ensure_commands(&sandbox, &runner.documentation_contracts_commands("-j 2"))?;
    let environment = sandbox.environment()?;
    ensure!(
        environment
            .iter()
            .all(|record| record.contains("RUSTFLAGS=-D warnings")),
        "the documentation gate must deny warnings: {environment:#?}"
    );
    Ok(())
}

/// A failing documentation test fails the gate with no later command attempted.
#[rstest]
#[case::nextest(Runner::Nextest)]
#[case::cargo_test_fallback(Runner::CargoTest)]
fn documentation_contracts_stops_at_the_first_failing_step(#[case] runner: Runner) -> Result<()> {
    let mut sandbox = documentation_sandbox(runner)?;
    sandbox.set_var(ChildVar::FailingCommand, runner.failing_command());
    ensure!(
        !run(&sandbox, Gate::DocumentationContracts)?,
        "test-documentation-contracts must fail when {runner:?} fails"
    );
    ensure_commands(&sandbox, &runner.documentation_contracts_commands(""))
}
