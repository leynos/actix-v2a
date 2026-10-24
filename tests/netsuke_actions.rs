//! Behavioural checks for the `Netsukefile` gate actions.
//!
//! The repository has no `Makefile`: every gate is a Netsuke action, so the
//! manifest's recipes are the contract contributors and CI rely on. These tests
//! drive the real `netsuke` binary against the committed manifest, with fake
//! tools that log their arguments and selected environment, and assert the
//! observable result: which commands run, in what order, with which flags, and
//! whether a failure stops the gate.

#[path = "netsuke_actions/sandbox.rs"]
mod sandbox;

use anyhow::{Result, ensure};
use rstest::rstest;
use sandbox::{ActionSandbox, ToolHome};

const CHECK_FMT: [&str; 2] = [
    "cargo\tfmt\t--all\t--\t--check",
    concat!(
        "mdtablefix\t--check\t--git\t--include-untracked",
        "\t--wrap\t--renumber\t--breaks\t--ellipsis\t--fences",
    ),
];

const LINT: [&str; 3] = [
    "cargo\tdoc\t--no-deps",
    "cargo\tclippy\t--all-targets\t--all-features\t--\t-D\twarnings",
    "whitaker\t--all\t--\t--all-targets\t--all-features",
];

const NEXTEST: [&str; 2] = [
    "cargo\tnextest\trun\t--all-targets\t--all-features",
    "cargo\ttest\t--doc\t--workspace\t--all-features",
];

const CARGO_TEST: [&str; 1] = ["cargo\ttest\t--all-targets\t--all-features"];

/// Runs `action` and reports whether Netsuke exited successfully.
fn run(sandbox: &ActionSandbox, action: &str) -> Result<bool> {
    let output = sandbox.netsuke(&["build", action])?;
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

#[rstest]
#[case::nextest(true, &NEXTEST)]
#[case::cargo_test_fallback(false, &CARGO_TEST)]
fn test_selects_the_available_runner(
    #[case] has_nextest: bool,
    #[case] expected: &[&str],
) -> Result<()> {
    let sandbox = ActionSandbox::with_default_tools()?;
    if has_nextest {
        sandbox.install("cargo-nextest", ToolHome::Path)?;
    }
    ensure!(run(&sandbox, "test")?, "test action failed");
    ensure_commands(&sandbox, expected)?;
    let environment = sandbox.environment()?;
    ensure!(
        environment
            .iter()
            .all(|record| record.contains("RUSTFLAGS=-D warnings")),
        "every test command must deny warnings: {environment:#?}"
    );
    Ok(())
}

#[rstest]
fn lint_runs_rustdoc_clippy_then_whitaker_with_warnings_denied() -> Result<()> {
    let sandbox = ActionSandbox::with_default_tools()?;
    ensure!(run(&sandbox, "lint")?, "lint action failed");
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

#[rstest]
#[case::rustdoc_fails("cargo doc", &LINT[..1])]
#[case::clippy_fails("cargo clippy", &LINT[..2])]
fn lint_stops_at_the_first_failing_step(
    #[case] failing: &str,
    #[case] expected: &[&str],
) -> Result<()> {
    let mut sandbox = ActionSandbox::with_default_tools()?;
    sandbox.set_var("FAILING_COMMAND", failing);
    ensure!(
        !run(&sandbox, "lint")?,
        "lint must fail when {failing} fails"
    );
    ensure_commands(&sandbox, expected)
}

#[rstest]
fn lint_fails_when_whitaker_is_missing() -> Result<()> {
    let sandbox = ActionSandbox::empty()?;
    sandbox.install("cargo", ToolHome::Path)?;
    ensure!(!run(&sandbox, "lint")?, "lint must fail without whitaker");
    ensure_commands(&sandbox, &LINT[..2])
}

#[rstest]
fn check_fmt_checks_rust_then_markdown_tables() -> Result<()> {
    let sandbox = ActionSandbox::with_default_tools()?;
    ensure!(run(&sandbox, "check-fmt")?, "check-fmt action failed");
    ensure_commands(&sandbox, &CHECK_FMT)
}

#[rstest]
#[case::spelling(
    "spelling",
    "uv\ttool\trun\t--python\t3.14\t--from\tgit+https://github.com/leynos/typos-config-builder.git@",
    "\ttypos-config-builder\tgate\t--repository\t."
)]
#[case::workflow_contracts(
    "test-workflow-contracts",
    "uv\ttool\trun\t--python\t3.13\t--from\tgit+https://github.com/leynos/shared-actions@",
    "#subdirectory=packages/cv005-contracts\tcv005-contracts\tcheck\t--repository\t."
)]
fn uv_gates_run_the_pinned_tool_with_a_repository_cache(
    #[case] action: &str,
    #[case] prefix: &str,
    #[case] suffix: &str,
) -> Result<()> {
    let sandbox = ActionSandbox::with_default_tools()?;
    ensure!(run(&sandbox, action)?, "{action} action failed");
    let commands = sandbox.commands()?;
    ensure!(
        matches!(commands.as_slice(), [only] if only.starts_with(prefix) && only.ends_with(suffix)),
        "{action} ran unexpected commands: {commands:#?}"
    );
    let environment = sandbox.environment()?;
    ensure!(
        environment
            .iter()
            .all(|record| record.contains("UV_CACHE_DIR=.uv-cache")),
        "{action} must keep uv's cache in the repository: {environment:#?}"
    );
    Ok(())
}

#[rstest]
fn all_runs_every_gate_serially_in_declared_order() -> Result<()> {
    let sandbox = ActionSandbox::with_default_tools()?;
    sandbox.install("cargo-nextest", ToolHome::Path)?;
    ensure!(run(&sandbox, "all")?, "all action failed");
    let commands = sandbox.commands()?;
    let gates: Vec<&str> = commands.iter().map(|line| gate_name(line)).collect();
    let expected = [
        "spelling",
        "workflow-contracts",
        "check-fmt",
        "check-fmt",
        "lint",
        "lint",
        "lint",
        "test",
        "test",
    ];
    ensure!(gates == expected, "gate order differs: {commands:#?}");
    Ok(())
}

#[rstest]
fn all_stops_before_later_gates_when_one_fails() -> Result<()> {
    let mut sandbox = ActionSandbox::with_default_tools()?;
    sandbox.set_var("FAILING_COMMAND", "mdtablefix");
    ensure!(!run(&sandbox, "all")?, "all must fail when check-fmt fails");
    let commands = sandbox.commands()?;
    ensure!(
        commands.last().map(String::as_str) == CHECK_FMT.last().copied(),
        "check-fmt must be the last gate attempted: {commands:#?}"
    );
    ensure!(
        !commands
            .iter()
            .any(|line| line.starts_with("whitaker") || line.starts_with("cargo\tclippy")),
        "lint must not start after check-fmt fails: {commands:#?}"
    );
    Ok(())
}

#[rstest]
fn actions_prepend_user_tool_directories_to_path() -> Result<()> {
    let sandbox = ActionSandbox::empty()?;
    sandbox.install("nixie", ToolHome::LocalBin)?;
    ensure!(
        run(&sandbox, "nixie")?,
        "nixie in $HOME/.local/bin must be found"
    );
    ensure_commands(&sandbox, &["nixie\t--no-sandbox"])
}

#[rstest]
fn cargo_and_rust_flag_overrides_reach_the_recipe() -> Result<()> {
    let mut sandbox = ActionSandbox::empty()?;
    sandbox.install("pinned-cargo", ToolHome::Path)?;
    sandbox.set_var("CARGO", "pinned-cargo");
    sandbox.set_var("RUST_FLAGS", "--cfg sandbox");
    ensure!(run(&sandbox, "typecheck")?, "typecheck action failed");
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

/// Names the gate that issues a logged `tool<TAB>arg...` command line.
fn gate_name(line: &str) -> &'static str {
    let mut words = line.split('\t');
    let invocation = (words.next(), words.next());
    match invocation {
        (Some("uv"), _) if line.contains("typos-config-builder") => "spelling",
        (Some("uv"), _) if line.contains("cv005-contracts") => "workflow-contracts",
        (Some("cargo"), Some("fmt")) | (Some("mdtablefix"), _) => "check-fmt",
        (Some("cargo"), Some("doc" | "clippy")) | (Some("whitaker"), _) => "lint",
        (Some("cargo"), Some("nextest" | "test")) => "test",
        _ => "unknown",
    }
}
