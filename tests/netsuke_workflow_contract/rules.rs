//! Judges whether workflow jobs drive the gates through the pinned Netsuke.
//!
//! The judgement works on parsed workflows so it can be exercised against
//! constructed jobs before it is applied to the repository's own files. A job
//! is read as one ordered stream of shell command segments: every `run` line,
//! split at `;`, `|` and `&`, so an install and a use in the same script are
//! judged in the order the shell runs them.

use std::collections::BTreeSet;

use serde_norway::{Mapping, Value};

use super::reader;

/// The `netsuke-build` release the manifest's recipe syntax requires.
pub const NETSUKE_VERSION: &str = "0.1.0-beta4";

/// The shared action that runs the test suite under coverage. The suite
/// includes the `Netsukefile` action tests, which execute `netsuke`.
const COVERAGE_ACTION: &str = "leynos/shared-actions/.github/actions/generate-coverage@";

/// The install command for the pinned release, as the workflows write it.
const PINNED_INSTALL: &str = "netsuke-build --version \"=${NETSUKE_VERSION}\"";

/// Cargo subcommands that run the test suite, and so execute `netsuke`.
const TEST_SUBCOMMANDS: [&str; 3] = ["test", "nextest", "llvm-cov"];

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

/// Reduces one step mapping to its `run`, `uses`, and `NETSUKE_VERSION`.
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

/// One simple shell command: the words between `;`, `|` and `&`.
struct Segment<'a>(Vec<&'a str>);

impl Segment<'_> {
    /// Whether this command runs `netsuke`, other than a `--version` probe.
    fn runs_netsuke(&self) -> bool {
        let names_netsuke = self
            .0
            .iter()
            .any(|word| *word == "netsuke" || word.ends_with("/netsuke"));
        names_netsuke && !self.0.contains(&"--version")
    }

    /// Whether this command runs the test suite through Cargo.
    fn runs_test_suite(&self) -> bool {
        self.0
            .windows(2)
            .any(|pair| matches!(pair, ["cargo", sub] if TEST_SUBCOMMANDS.contains(sub)))
    }

    /// The action a `netsuke ... build <action>` command builds, if any.
    fn built_action(&self) -> Option<String> {
        let mut after_build = self.0.iter().skip_while(|word| **word != "build");
        after_build.next()?;
        after_build.next().map(|action| (*action).to_owned())
    }
}

/// Splits a shell line into the simple commands the shell runs in order.
fn segments(line: &str) -> impl Iterator<Item = Segment<'_>> {
    line.split([';', '|', '&'])
        .map(|segment| Segment(segment.split_whitespace().collect()))
        .filter(|segment| !segment.0.is_empty())
}

/// One thing a job does, in order: a step's `uses` action, or one simple
/// command from a `run` line. `text` is the whole line, for diagnostics.
struct Command<'a> {
    text: &'a str,
    segment: Segment<'a>,
    version: Option<&'a str>,
    is_coverage: bool,
}

impl Command<'_> {
    /// Whether this command needs Netsuke and Ninja installed already.
    fn needs_netsuke(&self) -> bool {
        self.is_coverage || self.segment.runs_netsuke() || self.segment.runs_test_suite()
    }

    /// Whether this command installs the pinned `netsuke-build` release.
    fn installs_pinned_netsuke(&self) -> bool {
        self.version == Some(NETSUKE_VERSION) && self.segment.0.join(" ").contains(PINNED_INSTALL)
    }

    /// Whether this command installs Ninja.
    fn installs_ninja(&self) -> bool { self.segment.0.contains(&"ninja-build") }

    /// Describes this command running before the installs.
    fn premature(&self) -> String {
        format!(
            "`{}` runs before netsuke-build {NETSUKE_VERSION} and Ninja are installed",
            self.text.trim()
        )
    }
}

/// Every command in the steps, in the order the job runs them: each step's
/// `uses` action, then each simple command of each `run` line.
fn commands(steps: &[Step]) -> impl Iterator<Item = Command<'_>> {
    steps.iter().flat_map(|step| {
        let uses = (!step.uses.is_empty()).then(|| Command {
            text: &step.uses,
            segment: Segment(Vec::new()),
            version: None,
            is_coverage: step.uses.starts_with(COVERAGE_ACTION),
        });
        let runs = step.run.lines().flat_map(move |line| {
            segments(line).map(move |segment| Command {
                text: line,
                segment,
                version: step.version.as_deref(),
                is_coverage: false,
            })
        });
        uses.into_iter().chain(runs)
    })
}

/// Install progress through a job, in shell order.
#[derive(Default)]
struct Installed {
    netsuke: bool,
    ninja: bool,
}

impl Installed {
    /// Whether both the pinned Netsuke and Ninja are available.
    const fn is_ready(&self) -> bool { self.netsuke && self.ninja }

    /// Records any install that `command` performs.
    fn observe(&mut self, command: &Command<'_>) {
        self.netsuke |= command.installs_pinned_netsuke();
        self.ninja |= command.installs_ninja();
    }
}

/// Describes each use of Netsuke, direct or through the test suite, that comes
/// before netsuke-build 0.1.0-beta4 and Ninja are both installed.
pub fn missing_installs(steps: &[Step]) -> Vec<String> {
    let mut installed = Installed::default();
    commands(steps)
        .filter_map(|command| {
            // Judge each simple command against the installs that precede
            // it, so a use earlier on the install's own line still counts.
            let is_premature = command.needs_netsuke() && !installed.is_ready();
            installed.observe(&command);
            is_premature.then(|| command.premature())
        })
        .collect()
}

/// Describes each step that still calls `make`, which no longer exists.
pub fn make_calls(steps: &[Step]) -> Vec<String> {
    steps
        .iter()
        .flat_map(|step| step.run.lines())
        .filter(|line| segments(line).any(|segment| segment.0.first() == Some(&"make")))
        .map(|line| {
            format!(
                "`{}` calls make; the repository has no Makefile",
                line.trim()
            )
        })
        .collect()
}

/// Returns each action named by `netsuke ... build <action>` in the steps,
/// including every invocation on a line that chains several.
pub fn built_actions(steps: &[Step]) -> BTreeSet<String> {
    steps
        .iter()
        .flat_map(|step| step.run.lines())
        .flat_map(segments)
        .filter(Segment::runs_netsuke)
        .filter_map(|segment| segment.built_action())
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
