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

#[path = "runner_placement/reader.rs"]
mod reader;
#[path = "netsuke_workflow_contract/rules.rs"]
mod rules;

use anyhow::{Context, Result, ensure};
use rstest::rstest;
use rules::Step;

/// Gates CI must run through Netsuke; `test` runs inside the coverage action.
const CI_GATES: [&str; 5] = [
    "check-fmt",
    "lint",
    "nixie",
    "spelling",
    "test-workflow-contracts",
];

fn job_steps(yaml: &str) -> Result<Vec<Step>> {
    let workflow = reader::parse("constructed.yml", yaml)?;
    let (_, job) = reader::jobs(&workflow)
        .into_iter()
        .next()
        .context("constructed workflow has a job")?;
    Ok(rules::steps(job))
}

const INSTALLS: &str = r#"
      - env: { NETSUKE_VERSION: 0.1.0-beta4 }
        run: cargo install --locked netsuke-build --version "=${NETSUKE_VERSION}"
      - run: sudo apt-get install -y ninja-build
"#;

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
fn install_rule_flags_gates_without_pinned_netsuke(
    #[case] steps_yaml: &str,
    #[case] expected_problems: usize,
) -> Result<()> {
    let steps = job_steps(&format!("jobs:\n  gate:\n    steps:\n{steps_yaml}"))?;
    let problems = rules::missing_installs(&steps);
    ensure!(problems.len() == expected_problems, "{problems:#?}");
    Ok(())
}

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

#[rstest]
fn built_actions_reads_verbose_and_piped_invocations() -> Result<()> {
    let steps = job_steps(concat!(
        "jobs:\n  gate:\n    steps:\n",
        "      - run: >-\n",
        "          set -o pipefail;\n",
        "          netsuke --verbose build lint 2>&1 | tee /tmp/lint.out\n",
        "      - run: netsuke build nixie\n",
    ))?;
    let actions = rules::built_actions(&steps);
    ensure!(
        actions.iter().map(String::as_str).eq(["lint", "nixie"]),
        "{actions:?}"
    );
    Ok(())
}

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
