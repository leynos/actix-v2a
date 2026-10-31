//! Holds CI to the Netsuke-only build: every gate runs through `netsuke`.
//!
//! The repository has no `Makefile`, and the `Netsukefile` relies on the
//! netsuke-build v0.1.0-beta4 recipe contract, under which an older Netsuke
//! mis-escapes shell dollars. So any job that runs a gate, or runs the test
//! suite (whose `Netsukefile` action tests execute `netsuke`), must install
//! that release and Ninja first, must not call `make`, and must name only
//! actions the manifest declares. The rules in [`rules`] are driven against
//! constructed jobs before they are applied to the real workflows, because a
//! check run only over correct workflows would pass whether or not it detected
//! anything.

#[path = "netsuke_workflow_contract/properties.rs"]
mod properties;
#[path = "runner_placement/reader.rs"]
mod reader;
#[path = "netsuke_workflow_contract/rules.rs"]
mod rules;

use anyhow::{Context, Result, ensure};
use rstest::rstest;
use rules::Step;

/// Gates CI must run through Netsuke; `test` runs inside the coverage action.
const CI_GATES: [&str; 6] = [
    "check-fmt",
    "lint",
    "nixie",
    "spelling",
    "test-documentation-contracts",
    "test-workflow-contracts",
];

/// Parses the steps of the single job in a constructed workflow.
fn job_steps(yaml: &str) -> Result<Vec<Step>> {
    let workflow = reader::parse("constructed.yml", yaml)?;
    let (_, job) = reader::jobs(&workflow)
        .into_iter()
        .next()
        .context("constructed workflow has a job")?;
    Ok(rules::steps(job))
}

/// Wraps a shell script as the only `run` step of a constructed job.
fn run_step(script: &str) -> String {
    let indented = script.lines().fold(String::new(), |mut body, line| {
        body.push_str("          ");
        body.push_str(line);
        body.push('\n');
        body
    });
    format!("jobs:\n  gate:\n    steps:\n      - run: |\n{indented}")
}

/// Installs Ninja and the pinned Netsuke, as two steps.
const INSTALLS: &str = r#"
      - env: { NETSUKE_VERSION: 0.1.0-beta4 }
        run: cargo install --locked netsuke-build --version "=${NETSUKE_VERSION}"
      - run: sudo apt-get install -y ninja-build
"#;

/// The install rule reports each use of Netsuke, direct or through the test
/// suite, that precedes the pinned release or Ninja.
#[rstest]
#[case::gate_after_installs(&format!("{INSTALLS}      - run: netsuke build lint\n"), 0)]
#[case::gate_before_installs(&format!("      - run: netsuke build lint\n{INSTALLS}"), 1)]
#[case::coverage_without_netsuke(
    "      - uses: leynos/shared-actions/.github/actions/generate-coverage@abc\n",
    1
)]
#[case::wrong_netsuke_version(
    &format!("{}      - run: netsuke build lint\n", INSTALLS.replace("beta4", "beta3")),
    1
)]
#[case::missing_ninja(
    concat!(
        "      - env: { NETSUKE_VERSION: 0.1.0-beta4 }\n",
        "        run: cargo install --locked netsuke-build --version \"=${NETSUKE_VERSION}\"\n",
        "      - run: netsuke build lint\n",
    ),
    1
)]
#[case::cargo_test_before_installs(&format!("      - run: cargo test --workspace\n{INSTALLS}"), 1)]
#[case::nextest_before_installs(&format!("      - run: cargo nextest run\n{INSTALLS}"), 1)]
#[case::netsuke_before_its_install_in_one_script(
    concat!(
        "      - run: sudo apt-get install -y ninja-build\n",
        "      - env: { NETSUKE_VERSION: 0.1.0-beta4 }\n",
        "        run: |\n",
        "          netsuke build lint\n",
        "          cargo install --locked netsuke-build --version \"=${NETSUKE_VERSION}\"\n",
    ),
    1
)]
#[case::use_before_install_on_one_line_semicolon(
    concat!(
        "      - run: sudo apt-get install -y ninja-build\n",
        "      - env: { NETSUKE_VERSION: 0.1.0-beta4 }\n",
        "        run: netsuke build lint; cargo install --locked netsuke-build --version \"=${NETSUKE_VERSION}\"\n",
    ),
    1
)]
#[case::use_before_install_on_one_line_and(
    concat!(
        "      - run: sudo apt-get install -y ninja-build\n",
        "      - env: { NETSUKE_VERSION: 0.1.0-beta4 }\n",
        "        run: netsuke build lint && cargo install --locked netsuke-build --version \"=${NETSUKE_VERSION}\"\n",
    ),
    1
)]
#[case::install_before_use_on_one_line(
    concat!(
        "      - run: sudo apt-get install -y ninja-build\n",
        "      - env: { NETSUKE_VERSION: 0.1.0-beta4 }\n",
        "        run: cargo install --locked netsuke-build --version \"=${NETSUKE_VERSION}\" && netsuke build lint\n",
    ),
    0
)]
#[case::installer_version_probe_is_exempt(
    concat!(
        "      - run: sudo apt-get install -y ninja-build\n",
        "      - env: { NETSUKE_VERSION: 0.1.0-beta4 }\n",
        "        run: |\n",
        "          if netsuke --version | grep -q beta4; then exit 0; fi\n",
        "          cargo install --locked netsuke-build --version \"=${NETSUKE_VERSION}\"\n",
        "      - run: netsuke build lint\n",
    ),
    0
)]
fn install_rule_flags_gates_without_pinned_netsuke(
    #[case] steps_yaml: &str,
    #[case] expected_problems: usize,
) -> Result<()> {
    let steps = job_steps(&format!("jobs:\n  gate:\n    steps:\n{steps_yaml}"))?;
    let problems = rules::missing_installs(&steps);
    ensure!(problems.len() == expected_problems, "{problems:#?}");
    Ok(())
}

/// The make rule reports a `make` command, not the word in other text.
#[rstest]
#[case::make_gate("      - run: make lint\n", 1)]
#[case::netsuke_gate("      - run: netsuke build lint\n", 0)]
#[case::make_in_prose("      - run: echo 'make sure'\n", 0)]
fn make_rule_flags_make_invocations(
    #[case] steps_yaml: &str,
    #[case] expected: usize,
) -> Result<()> {
    let steps = job_steps(&format!("jobs:\n  gate:\n    steps:\n{steps_yaml}"))?;
    let calls = rules::make_calls(&steps);
    ensure!(calls.len() == expected, "{calls:#?}");
    Ok(())
}

/// The pipefail rule reports a Netsuke gate whose failure `tee` would mask.
#[rstest]
#[case::protected("set -o pipefail;\nnetsuke build lint 2>&1 | tee /tmp/lint.out\n", 0)]
#[case::extended_pipefail("set -euo pipefail\nnetsuke build lint 2>&1 | tee /tmp/lint.out\n", 0)]
#[case::missing_pipefail("netsuke build lint 2>&1 | tee /tmp/lint.out\n", 1)]
#[case::pipefail_after_the_pipe(
    "netsuke build lint 2>&1 | tee /tmp/lint.out\nset -o pipefail\n",
    1
)]
#[case::pipefail_disabled(
    "set -o pipefail\nset +o pipefail\nnetsuke build lint 2>&1 | tee /tmp/lint.out\n",
    1
)]
#[case::pipefail_disabled_with_options(
    "set -euo pipefail\nset +euo pipefail\nnetsuke build lint 2>&1 | tee /tmp/lint.out\n",
    1
)]
#[case::not_netsuke("capture --verbose | tee /tmp/capture.out\n", 0)]
fn pipefail_rule_flags_unprotected_netsuke_pipelines(
    #[case] script: &str,
    #[case] expected: usize,
) -> Result<()> {
    let steps = job_steps(&run_step(script))?;
    let problems = rules::unprotected_pipelines(&steps);
    ensure!(problems.len() == expected, "{problems:#?}");
    Ok(())
}

/// Action extraction reads verbose, piped, and chained invocations, including
/// every `netsuke build` on one line.
#[rstest]
fn built_actions_reads_verbose_piped_and_chained_invocations() -> Result<()> {
    let steps = job_steps(concat!(
        "jobs:\n  gate:\n    steps:\n",
        "      - run: >-\n",
        "          set -o pipefail;\n",
        "          netsuke --verbose build lint 2>&1 | tee /tmp/lint.out\n",
        "      - run: netsuke build nixie && netsuke build spelling\n",
        "      - run: cargo build --release\n",
    ))?;
    let actions = rules::built_actions(&steps);
    ensure!(
        actions
            .iter()
            .map(String::as_str)
            .eq(["lint", "nixie", "spelling"]),
        "{actions:?}"
    );
    Ok(())
}

/// No repository workflow uses Netsuke before installing it, or calls `make`.
#[rstest]
fn every_workflow_job_installs_netsuke_before_using_it() -> Result<()> {
    let workflows = reader::workflows()?;
    let problems: Vec<String> = workflows
        .iter()
        .flat_map(|(name, workflow)| {
            reader::jobs(workflow)
                .into_iter()
                .flat_map(move |(id, job)| {
                    let steps = rules::steps(job);
                    let mut found = rules::missing_installs(&steps);
                    found.extend(rules::make_calls(&steps));
                    found
                        .into_iter()
                        .map(move |problem| format!("{name} {id}: {problem}"))
                })
        })
        .collect();
    ensure!(problems.is_empty(), "{problems:#?}");
    Ok(())
}

/// CI runs every gate, and only actions the `Netsukefile` declares.
#[rstest]
fn ci_runs_every_gate_through_declared_netsuke_actions() -> Result<()> {
    let workflows = reader::workflows()?;
    let ci = workflows.get("ci.yml").context("ci.yml exists")?;
    let built: Vec<String> = reader::jobs(ci)
        .into_iter()
        .flat_map(|(_, job)| rules::built_actions(&rules::steps(job)))
        .collect();
    let manifest = reader::parse("Netsukefile", include_str!("../Netsukefile"))?;
    let declared = rules::declared_actions(&manifest);
    let undeclared: Vec<&String> = built
        .iter()
        .filter(|action| !declared.contains(*action))
        .collect();
    ensure!(
        undeclared.is_empty(),
        "CI builds undeclared actions: {undeclared:?}"
    );
    let missing: Vec<&str> = CI_GATES
        .into_iter()
        .filter(|gate| !built.iter().any(|action| action == gate))
        .collect();
    ensure!(missing.is_empty(), "CI does not run gates {missing:?}");
    Ok(())
}

/// No workflow lets `tee` mask a Netsuke gate failure.
#[rstest]
fn every_netsuke_pipeline_enables_pipefail_before_its_tee() -> Result<()> {
    let workflows = reader::workflows()?;
    let problems: Vec<String> = workflows
        .iter()
        .flat_map(|(name, workflow)| {
            reader::jobs(workflow)
                .into_iter()
                .flat_map(move |(id, job)| {
                    rules::unprotected_pipelines(&rules::steps(job))
                        .into_iter()
                        .map(move |problem| format!("{name} {id}: {problem}"))
                })
        })
        .collect();
    ensure!(problems.is_empty(), "{problems:#?}");
    Ok(())
}

/// The documentation gate's step runs through a protected `tee`, so the gate's
/// own failure, not `tee`'s success, decides whether the step passes.
#[rstest]
fn ci_documentation_gate_pipes_through_a_protected_tee() -> Result<()> {
    let workflows = reader::workflows()?;
    let ci = workflows.get("ci.yml").context("ci.yml exists")?;
    let steps: Vec<Step> = reader::jobs(ci)
        .into_iter()
        .flat_map(|(_, job)| rules::steps(job))
        .collect();
    let step = steps
        .iter()
        .find(|step| step.run.contains("build test-documentation-contracts"))
        .context("CI runs the documentation gate")?;
    ensure!(
        rules::pipes_netsuke_into_tee(&step.run),
        "the documentation gate must pipe its output to tee: {}",
        step.run
    );
    ensure!(
        rules::enables_pipefail(&step.run),
        "the documentation gate must enable pipefail before the pipe: {}",
        step.run
    );
    Ok(())
}
