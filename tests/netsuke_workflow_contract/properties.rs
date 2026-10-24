//! Property tests for the install-order rule over generated jobs.
//!
//! The rule scans a job once, tracking which installs it has seen. These
//! properties state the same requirement by position instead: a command that
//! needs Netsuke is reported exactly when it comes before the later of the
//! first pinned-Netsuke install and the first Ninja install. Generated commands
//! are laid out across steps, some sharing one run line, so the scan must also
//! respect order within a line. Agreement between the incremental scan and the
//! positional statement, over arbitrary orderings, line layouts, and irrelevant
//! padding, is the evidence that the scan's state is right.

use proptest::prelude::*;

use super::rules::{self, Step};

/// The kinds of step that matter to the install-order rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    InstallNetsuke,
    InstallNinja,
    Gate,
    Coverage,
    TestRun,
    Unrelated,
}

impl Kind {
    /// Whether the rule requires both installs before this command.
    const fn needs_netsuke(self) -> bool {
        matches!(self, Self::Gate | Self::Coverage | Self::TestRun)
    }

    /// The shell command for this kind, or `None` for the coverage action,
    /// which is a `uses` step rather than a command.
    const fn command(self) -> Option<&'static str> {
        match self {
            Self::InstallNetsuke => {
                Some("cargo install --locked netsuke-build --version \"=${NETSUKE_VERSION}\"")
            }
            Self::InstallNinja => Some("sudo apt-get install -y ninja-build"),
            Self::Gate => Some("netsuke build lint"),
            Self::Coverage => None,
            Self::TestRun => Some("cargo nextest run --workspace"),
            Self::Unrelated => Some("echo unrelated"),
        }
    }
}

/// A generated command and whether it shares the previous step's run line.
type Placed = (Kind, bool);

/// Generates any kind of command.
fn kind() -> impl Strategy<Value = Kind> {
    prop_oneof![
        Just(Kind::InstallNetsuke),
        Just(Kind::InstallNinja),
        Just(Kind::Gate),
        Just(Kind::Coverage),
        Just(Kind::TestRun),
        Just(Kind::Unrelated),
    ]
}

/// Generates a job as placed commands, some sharing run lines.
fn job() -> impl Strategy<Value = Vec<Placed>> {
    prop::collection::vec((kind(), any::<bool>()), 0..16)
}

/// Lays placed commands out as workflow steps. A command marked to share a
/// line joins the previous step's run line with `; `; the coverage action is
/// always a step of its own. Every run step carries the pinned
/// `NETSUKE_VERSION`, which only an install command reads.
fn steps(job: &[Placed]) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    for &(kind, shares_line) in job {
        match (kind.command(), steps.last_mut()) {
            (Some(command), Some(last)) if shares_line && !last.run.is_empty() => {
                last.run.push_str("; ");
                last.run.push_str(command);
            }
            (Some(command), _) => steps.push(Step {
                run: command.to_owned(),
                version: Some(rules::NETSUKE_VERSION.to_owned()),
                ..Step::default()
            }),
            (None, _) => steps.push(Step {
                uses: "leynos/shared-actions/.github/actions/generate-coverage@abc".to_owned(),
                ..Step::default()
            }),
        }
    }
    steps
}

/// The number of problems the rule should report for `job`, by position in
/// the order the shell runs the commands, regardless of line layout.
fn expected_problems(job: &[Placed]) -> usize {
    let first = |wanted| job.iter().position(|(kind, _)| *kind == wanted);
    let ready = first(Kind::InstallNetsuke)
        .zip(first(Kind::InstallNinja))
        .map_or(job.len(), |(netsuke, ninja)| netsuke.max(ninja));
    job.iter()
        .take(ready)
        .filter(|(kind, _)| kind.needs_netsuke())
        .count()
}

/// Runs the rule over the steps `job` lays out.
fn reported_problems(job: &[Placed]) -> usize { rules::missing_installs(&steps(job)).len() }

proptest! {
    /// The scan reports exactly the needing commands before both installs,
    /// whether commands share run lines or not.
    #[test]
    fn reports_needing_commands_before_both_installs(job in job()) {
        prop_assert_eq!(reported_problems(&job), expected_problems(&job));
    }

    /// Unrelated commands never change what the rule reports.
    #[test]
    fn unrelated_commands_do_not_change_the_report(
        job in job(),
        position in any::<prop::sample::Index>(),
        shares_line in any::<bool>(),
    ) {
        let mut padded = job.clone();
        padded.insert(position.index(job.len() + 1), (Kind::Unrelated, shares_line));
        prop_assert_eq!(reported_problems(&padded), reported_problems(&job));
    }
}
