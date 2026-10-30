//! Executable contracts for examples in the published user's guide.
//!
//! Every fenced example is loaded from `docs/users-guide.md` through the shared
//! loader. Rust fences are compiled and run in an isolated Cargo package, while
//! console and operator-output fences have their documented text checked
//! against the behaviour they claim.

#[path = "documentation_examples/mod.rs"]
mod documentation_examples;
#[path = "documentation_examples/workspace.rs"]
mod example_workspace;
#[path = "documentation_examples/parser_tests.rs"]
mod parser_tests;

use std::collections::BTreeSet;

use anyhow::{Result, ensure};
use documentation_examples::{DocumentedExample, documented_example, load_documented_examples};
use example_workspace::ExampleWorkspace;

/// The closed registry of user-guide examples. Adding a fence without choosing
/// its identifier and behavioural contract fails this list.
const EXPECTED_EXAMPLE_IDS: &[&str] = &[
    "guide-cache-control",
    "guide-classify-replay-errors",
    "guide-cursor-key",
    "guide-event-id",
    "guide-extract-actix-replay-cursor",
    "guide-heartbeat-policy",
    "guide-last-event-id-header",
    "guide-map-replay-cursor-error",
    "guide-openapi-params",
    "guide-operator-log-command",
    "guide-operator-log-output",
    "guide-pagination-query-params",
    "guide-render-comment-frame",
    "guide-render-event-frame",
    "guide-render-heartbeat-frame",
    "guide-render-stream-reset-frame",
];

#[test]
fn every_documented_fence_has_a_known_unique_identifier() -> Result<()> {
    let examples = load_documented_examples()?;
    let actual = examples
        .iter()
        .map(|example| example.id.as_str())
        .collect::<BTreeSet<_>>();
    let expected = EXPECTED_EXAMPLE_IDS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    ensure!(
        actual == expected,
        "documented example registry drifted\nexpected: {expected:#?}\nactual: {actual:#?}"
    );
    Ok(())
}

#[test]
fn documented_fences_use_supported_languages() -> Result<()> {
    let examples = load_documented_examples()?;
    for example in examples {
        ensure!(
            matches!(example.language.as_str(), "rust" | "sh" | "text"),
            "documented example '{}' has unsupported language '{}'",
            example.id,
            example.language
        );
    }
    Ok(())
}

/// Compiles and runs every Rust fence from the published guide.
#[test]
fn documented_rust_examples_compile_and_run() -> Result<()> {
    let rust_examples = load_documented_examples()?
        .iter()
        .filter(|example| example.language == "rust")
        .collect::<Vec<_>>();
    ensure!(
        rust_examples.len() == 14,
        "expected 14 documented Rust examples, got {}",
        rust_examples.len()
    );

    let workspace = ExampleWorkspace::new()?;
    for example in &rust_examples {
        workspace.add_binary(example)?;
    }
    workspace.build()?;
    for example in &rust_examples {
        workspace.run(&example.id)?;
    }
    Ok(())
}

#[test]
fn documented_operator_guidance_matches_the_tracing_contract() -> Result<()> {
    let command = documented_example("guide-operator-log-command")?;
    ensure!(
        command.language == "sh",
        "operator command should be a shell fence"
    );
    ensure!(
        command.body == "RUST_LOG=actix_v2a=error cargo run\n",
        "operator command drifted: {:?}",
        command.body
    );

    let output = documented_example("guide-operator-log-output")?;
    ensure!(
        output.language == "text",
        "operator output should be a text fence"
    );
    ensure!(
        output.body
            == concat!(
                "ERROR actix_v2a::sse::replay_cursor: ",
                "replay cursor header extraction failed\n",
                "  header_name=\"Last-Event-ID\"\n",
                "  error_variant=\"InvalidHeader\"\n",
            ),
        "operator output drifted: {:?}",
        output.body
    );
    Ok(())
}

#[test]
fn documented_rust_fences_are_non_empty() -> Result<()> {
    for example in rust_examples()? {
        ensure!(
            !example.body.trim().is_empty(),
            "documented Rust example '{}' should not be empty",
            example.id
        );
    }
    Ok(())
}

fn rust_examples() -> Result<Vec<&'static DocumentedExample>> {
    Ok(load_documented_examples()?
        .iter()
        .filter(|example| example.language == "rust")
        .collect())
}
