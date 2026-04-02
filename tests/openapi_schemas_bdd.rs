//! Behavioural tests for shared `OpenAPI` schema fragments.

use actix_v2a::{ErrorCodeSchema, ErrorSchema, ReplayMetadataSchema};
use rstest::fixture;
use rstest_bdd::{Slot, StepResult};
use rstest_bdd_macros::{ScenarioState, given, scenario, then};
use utoipa::openapi::{ComponentsBuilder, Info, OpenApi, OpenApiBuilder, Paths};

#[derive(Default, ScenarioState)]
struct World {
    document: Slot<utoipa::openapi::OpenApi>,
    json: Slot<String>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let has_document = self.document.with_ref(|_| true).unwrap_or(false);
        formatter
            .debug_struct("World")
            .field("document", &has_document)
            .field("json", &self.json)
            .finish()
    }
}

#[fixture]
fn world() -> World {
    // Keep an explicit body here so the fixture remains readable in test traces.
    World::default()
}

#[given("the shared OpenAPI schema document is generated")]
fn the_shared_openapi_schema_document_is_generated(world: &World) -> StepResult<(), String> {
    let document = build_shared_schema_document();
    let json = document
        .to_json()
        .map_err(|error| format!("OpenAPI document should serialize: {error}"))?;
    world.json.set(json);
    world.document.set(document);
    Ok(())
}

fn build_shared_schema_document() -> OpenApi {
    OpenApiBuilder::new()
        .info(Info::new("Shared schema fragments", "1.0.0"))
        .paths(Paths::new())
        .components(Some(
            ComponentsBuilder::new()
                .schema_from::<ErrorCodeSchema>()
                .schema_from::<ErrorSchema>()
                .schema_from::<ReplayMetadataSchema>()
                .build(),
        ))
        .build()
}

#[then("the components section contains the ErrorCode schema wrapper")]
fn the_components_section_contains_the_error_code_schema_wrapper(
    world: &World,
) -> StepResult<(), String> {
    assert_schema_registered(world, "crate.ErrorCode")
}

#[then("the components section contains the Error schema wrapper")]
fn the_components_section_contains_the_error_schema_wrapper(
    world: &World,
) -> StepResult<(), String> {
    assert_schema_registered(world, "crate.Error")
}

#[then("the components section contains the ReplayMetadata schema wrapper")]
fn the_components_section_contains_the_replay_metadata_schema_wrapper(
    world: &World,
) -> StepResult<(), String> {
    assert_schema_registered(world, "crate.idempotency.ReplayMetadata")
}

#[then("the Error schema exposes code message traceId and details fields")]
#[expect(
    clippy::panic_in_result_fn,
    reason = "BDD assertion steps should fail with direct assertion messages"
)]
fn the_error_schema_exposes_code_message_trace_id_and_details_fields(
    world: &World,
) -> StepResult<(), String> {
    let schema_json = schema_json(world, "crate.Error")?;
    for field in ["code", "message", "traceId", "details"] {
        assert!(schema_json.contains(field), "missing field {field}");
    }
    Ok(())
}

#[then("the ErrorCode schema enumerates the shared error codes")]
#[expect(
    clippy::panic_in_result_fn,
    reason = "BDD assertion steps should fail with direct assertion messages"
)]
fn the_error_code_schema_enumerates_the_shared_error_codes(
    world: &World,
) -> StepResult<(), String> {
    let schema_json = schema_json(world, "crate.ErrorCode")?;
    for variant in [
        "invalid_request",
        "unauthorized",
        "forbidden",
        "not_found",
        "conflict",
        "service_unavailable",
        "internal_error",
    ] {
        assert!(schema_json.contains(variant), "missing variant {variant}");
    }
    Ok(())
}

#[then("the ReplayMetadata schema exposes the replayed field")]
fn the_replay_metadata_schema_exposes_the_replayed_field(world: &World) -> StepResult<(), String> {
    let schema_json = schema_json(world, "crate.idempotency.ReplayMetadata")?;

    assert!(schema_json.contains("replayed"));
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "BDD assertion helpers should preserve assertion-style failures"
)]
fn assert_schema_registered(world: &World, schema_name: &str) -> StepResult<(), String> {
    let document = world
        .document
        .get()
        .ok_or_else(|| "OpenAPI document should be set".to_owned())?;
    let components = document
        .components
        .as_ref()
        .ok_or_else(|| "components should be present".to_owned())?;

    assert!(
        components.schemas.contains_key(schema_name),
        "schema {schema_name} should be registered"
    );
    Ok(())
}

fn schema_json(world: &World, schema_name: &str) -> StepResult<String, String> {
    let document = world
        .document
        .get()
        .ok_or_else(|| "OpenAPI document should be set".to_owned())?;
    let components = document
        .components
        .as_ref()
        .ok_or_else(|| "components should be present".to_owned())?;
    let schema = components
        .schemas
        .get(schema_name)
        .ok_or_else(|| "requested schema should be present".to_owned())?;

    serde_json::to_string(schema).map_err(|error| format!("schema should serialize: {error}"))
}

#[scenario(path = "tests/features/openapi_schemas.feature")]
fn shared_openapi_schema_fragments(world: World) { drop(world); }
