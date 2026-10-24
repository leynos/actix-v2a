//! Judges whether workflow jobs drive the gates through the pinned Netsuke.
//!
//! The judgement works on parsed workflows so it can be exercised against
//! constructed jobs before it is applied to the repository's own files.

use std::collections::BTreeSet;

use serde_norway::{Mapping, Value};

use super::reader;

/// The `netsuke-build` release the manifest's recipe syntax requires.
pub const NETSUKE_VERSION: &str = "0.1.0-beta4";

/// The shared action that runs the test suite under coverage. The suite
/// includes the `Netsukefile` action tests, which execute `netsuke`.
const COVERAGE_ACTION: &str = "leynos/shared-actions/.github/actions/generate-coverage@";

/// One workflow step, reduced to what the rules read.
#[derive(Debug, Default)]
pub struct Step {
    pub run: String,
    pub uses: String,
    pub version: Option<String>,
}

/// Reads a job's steps in order.
pub fn steps(job: &Mapping) -> Vec<Step> {
    reader::get(job, "steps")
        .and_then(Value::as_sequence)
        .map(|sequence| {
            sequence
                .iter()
                .filter_map(Value::as_mapping)
                .map(step)
                .collect()
        })
        .unwrap_or_default()
}

fn step(mapping: &Mapping) -> Step {
    let text = |key| {
        reader::get(mapping, key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let version = reader::get(mapping, "env")
        .and_then(Value::as_mapping)
        .and_then(|env| reader::get(env, "NETSUKE_VERSION"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    Step {
        run: text("run"),
        uses: text("uses"),
        version,
    }
}

/// Whether a step needs `netsuke` and Ninja to be installed already. The
/// install step's own `netsuke --version` probe does not count.
fn needs_netsuke(step: &Step) -> bool {
    !installs_pinned_netsuke(step)
        && (step.uses.starts_with(COVERAGE_ACTION)
            || netsuke_invocations(&step.run).next().is_some())
}

fn installs_pinned_netsuke(step: &Step) -> bool {
    step.run
        .contains("netsuke-build --version \"=${NETSUKE_VERSION}\"")
        && step.version.as_deref() == Some(NETSUKE_VERSION)
}

fn installs_ninja(step: &Step) -> bool { step.run.contains("ninja-build") }

/// Lines of a `run` script that invoke `netsuke`. Installing the
/// `netsuke-build` crate names the package, not the command, so it is not an
/// invocation.
fn netsuke_invocations(run: &str) -> impl Iterator<Item = &str> {
    run.lines().map(str::trim).filter(|line| {
        line.split_whitespace()
            .any(|word| word == "netsuke" || word.ends_with("/netsuke"))
    })
}

/// Describes each step that needs Netsuke before both installs have run.
pub fn missing_installs(steps: &[Step]) -> Vec<String> {
    let mut has_netsuke = false;
    let mut has_ninja = false;
    let mut problems = Vec::new();
    for step in steps {
        has_netsuke |= installs_pinned_netsuke(step);
        has_ninja |= installs_ninja(step);
        let is_installed = has_netsuke && has_ninja;
        if needs_netsuke(step) && !is_installed {
            problems.push(format!(
                "step `{}{}` runs before netsuke-build {NETSUKE_VERSION} and Ninja are installed",
                step.uses,
                step.run.lines().next().unwrap_or_default()
            ));
        }
    }
    problems
}

/// Describes each step that still calls `make`, which no longer exists.
pub fn make_calls(steps: &[Step]) -> Vec<String> {
    steps
        .iter()
        .flat_map(|step| step.run.lines())
        .filter(|line| line.split_whitespace().any(|word| word == "make"))
        .map(|line| {
            format!(
                "`{}` calls make; the repository has no Makefile",
                line.trim()
            )
        })
        .collect()
}

/// Returns each action named by `netsuke ... build <action>` in the steps.
pub fn built_actions(steps: &[Step]) -> BTreeSet<String> {
    steps
        .iter()
        .flat_map(|step| netsuke_invocations(&step.run))
        .filter_map(|line| {
            let mut words = line.split_whitespace().skip_while(|word| *word != "build");
            words.next()?;
            words.next().map(str::to_owned)
        })
        .collect()
}

/// Returns the action names the `Netsukefile` declares.
pub fn declared_actions(manifest: &Value) -> BTreeSet<String> {
    manifest
        .as_mapping()
        .and_then(|root| reader::get(root, "actions"))
        .and_then(Value::as_sequence)
        .map(|actions| {
            actions
                .iter()
                .filter_map(Value::as_mapping)
                .filter_map(|action| reader::get(action, "name")?.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}
