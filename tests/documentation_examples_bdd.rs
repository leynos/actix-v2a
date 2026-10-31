//! Behavioural scenarios for the published documentation examples.
//!
//! The scenarios share the loader used by the integration tests, so the
//! shipped Markdown is exercised rather than a copied fixture. They verify that
//! every user-facing fence is registered under a stable identifier and that the
//! documented operator output matches the fields the crate actually emits.

#[path = "documentation_examples/mod.rs"]
mod documentation_examples;
#[path = "support/sse_trace_buffer.rs"]
mod sse_trace_buffer;

use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use actix_v2a::extract_actix_replay_cursor;
use actix_web::http::header::{HeaderMap, HeaderName, HeaderValue};
use documentation_examples::{documented_example, is_valid_example_id, load_documented_examples};
use rstest::fixture;
use rstest_bdd::Slot;
use rstest_bdd_macros::{ScenarioState, given, scenario, then, when};
use sse_trace_buffer::LogBuffer;

type StepResult = Result<(), Box<dyn std::error::Error>>;

const OPERATOR_OUTPUT_EXAMPLE: &str = "guide-operator-log-output";

#[derive(Debug, Default, ScenarioState)]
struct World {
    example_ids: Slot<Vec<String>>,
    documented_fragments: Slot<Vec<String>>,
    log_buffer: Slot<LogBuffer>,
}

#[fixture]
fn world() -> World {
    // Keep the fixture explicit so scenario failures print a useful state type.
    let world = World::default();
    world.log_buffer.set(Arc::new(Mutex::new(Vec::new())));
    world
}

#[given("the published user-facing documentation is loaded")]
fn the_published_user_facing_documentation_is_loaded(world: &World) -> StepResult {
    let examples = load_documented_examples()
        .map_err(|error| format!("load documented examples: {error:#}"))?;
    let ids = examples
        .iter()
        .map(|example| example.id.clone())
        .collect::<Vec<_>>();
    world.example_ids.set(ids);
    Ok(())
}

#[then("every fenced example has a stable unique identifier")]
fn every_fenced_example_has_a_stable_unique_identifier(world: &World) -> StepResult {
    let ids = world
        .example_ids
        .get()
        .ok_or("documented example identifiers should be loaded")?;

    if ids.is_empty() {
        return Err("the user-facing documentation should declare at least one example".into());
    }
    if let Some(invalid) = ids.iter().find(|id| !is_valid_example_id(id)) {
        return Err(format!(
            "documented identifier {invalid:?} does not use the documented grammar"
        )
        .into());
    }

    let unique = ids.iter().collect::<BTreeSet<_>>();
    if unique.len() != ids.len() {
        return Err(format!("documented identifiers should be unique: {ids:?}").into());
    }
    Ok(())
}

#[given("the documented operator output for a malformed Last-Event-ID header")]
fn the_documented_operator_output_for_a_malformed_last_event_id_header(
    world: &World,
) -> StepResult {
    let example = documented_example(OPERATOR_OUTPUT_EXAMPLE)
        .map_err(|error| format!("load {OPERATOR_OUTPUT_EXAMPLE}: {error:#}"))?;
    let fragments = documented_fragments(&example.body)?;
    world.documented_fragments.set(fragments);
    Ok(())
}

#[when("a duplicate Last-Event-ID header is rejected by the Actix adapter")]
fn a_duplicate_last_event_id_header_is_rejected_by_the_actix_adapter(world: &World) {
    let mut headers = HeaderMap::new();
    let header_name = HeaderName::from_static("last-event-id");
    headers.append(header_name.clone(), HeaderValue::from_static("evt-001"));
    headers.append(header_name, HeaderValue::from_static("evt-002"));

    let result = sse_trace_buffer::with_tracing_buffer(&world.log_buffer, || {
        extract_actix_replay_cursor(&headers)
    });
    assert!(
        result.is_err(),
        "duplicate Last-Event-ID headers should be rejected"
    );
}

#[then("the emitted tracing event carries every documented field")]
fn the_emitted_tracing_event_carries_every_documented_field(world: &World) -> StepResult {
    let fragments = world
        .documented_fragments
        .get()
        .ok_or("documented operator output should be loaded")?;

    for fragment in &fragments {
        if !sse_trace_buffer::logs_contain(&world.log_buffer, fragment) {
            return Err(format!(
                "emitted tracing event is missing documented fragment: {fragment:?}"
            )
            .into());
        }
    }
    Ok(())
}

/// Split a documented event into the text fragments the trace buffer records.
///
/// The first line carries the level, target, and message; the buffer records
/// the message without the level or target. Later lines are the structured
/// fields exactly as `tracing` renders them.
fn documented_fragments(body: &str) -> Result<Vec<String>, String> {
    let mut lines = body.lines();
    let heading = lines
        .next()
        .ok_or_else(|| "documented operator output should have a heading".to_owned())?;
    let (_, message) = heading
        .split_once(": ")
        .ok_or_else(|| format!("documented heading should contain a message: {heading:?}"))?;

    let mut fragments = vec![message.to_owned()];
    for line in lines {
        let field = line.trim();
        if !field.is_empty() {
            fragments.push(field.to_owned());
        }
    }
    Ok(fragments)
}

#[scenario(path = "tests/features/documentation_examples.feature")]
fn documented_user_guide_examples(world: World) { drop(world); }
