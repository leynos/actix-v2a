//! Property tests for the install-order rule over generated jobs.
//!
//! The rule scans a job once, tracking which installs it has seen. These
//! properties state the same requirement by position instead: a step that
//! needs Netsuke is reported exactly when it comes before the later of the
//! first pinned-Netsuke install and the first Ninja install. Agreement between
//! the incremental scan and the positional statement, over arbitrary orderings
//! and irrelevant padding, is the evidence that the scan's state is right.

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
    /// Whether the rule requires both installs before this step.
    const fn needs_netsuke(self) -> bool {
        matches!(self, Self::Gate | Self::Coverage | Self::TestRun)
    }

    /// Builds a workflow step of this kind.
    fn step(self) -> Step {
        let run = |text: &str| Step {
            run: text.to_owned(),
            ..Step::default()
        };
        match self {
            Self::InstallNetsuke => Step {
                run: "cargo install --locked netsuke-build --version \"=${NETSUKE_VERSION}\""
                    .to_owned(),
                version: Some(rules::NETSUKE_VERSION.to_owned()),
                ..Step::default()
            },
            Self::InstallNinja => run("sudo apt-get install -y ninja-build"),
            Self::Gate => run("netsuke build lint"),
            Self::Coverage => Step {
                uses: "leynos/shared-actions/.github/actions/generate-coverage@abc".to_owned(),
                ..Step::default()
            },
            Self::TestRun => run("cargo nextest run --workspace"),
            Self::Unrelated => run("echo unrelated"),
        }
    }
}

/// Generates any step kind.
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

/// The number of problems the rule should report for `kinds`, by position.
fn expected_problems(kinds: &[Kind]) -> usize {
    let first = |wanted| kinds.iter().position(|kind| *kind == wanted);
    let ready = first(Kind::InstallNetsuke)
        .zip(first(Kind::InstallNinja))
        .map_or(kinds.len(), |(netsuke, ninja)| netsuke.max(ninja));
    kinds
        .iter()
        .take(ready)
        .filter(|kind| kind.needs_netsuke())
        .count()
}

/// Runs the rule over the steps `kinds` describe.
fn reported_problems(kinds: &[Kind]) -> usize {
    let steps: Vec<Step> = kinds.iter().map(|kind| kind.step()).collect();
    rules::missing_installs(&steps).len()
}

proptest! {
    /// The scan reports exactly the needing steps before both installs.
    #[test]
    fn reports_needing_steps_before_both_installs(
        kinds in prop::collection::vec(kind(), 0..16),
    ) {
        prop_assert_eq!(reported_problems(&kinds), expected_problems(&kinds));
    }

    /// Unrelated steps never change what the rule reports.
    #[test]
    fn unrelated_steps_do_not_change_the_report(
        kinds in prop::collection::vec(kind(), 0..16),
        position in any::<prop::sample::Index>(),
    ) {
        let mut padded = kinds.clone();
        padded.insert(position.index(kinds.len() + 1), Kind::Unrelated);
        prop_assert_eq!(reported_problems(&padded), reported_problems(&kinds));
    }
}
