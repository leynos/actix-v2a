//! Behavioural tests verifying pagination documentation invariants.

use actix_v2a::pagination::{
    Cursor,
    CursorError,
    DEFAULT_LIMIT,
    MAX_LIMIT,
    PageParams,
    PageParamsError,
};
use base64::Engine as _;
use rstest::fixture;
use rstest_bdd::Slot;
use rstest_bdd_macros::{ScenarioState, given, scenario, then, when};
use serde::{Deserialize, Serialize};

const OVERSIZED_CURSOR_LEN: usize = 8 * 1024 + 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FixtureKey {
    created_at: String,
    id: String,
}

#[derive(Debug, Default, ScenarioState)]
struct World {
    cursor_token: Slot<String>,
    decode_result: Slot<Result<Cursor<FixtureKey>, CursorError>>,
    page_params: Slot<PageParams>,
    page_params_result: Slot<Result<PageParams, PageParamsError>>,
    cursor_errors: Slot<Vec<CursorError>>,
}

#[fixture]
fn world() -> World {
    // Keep an explicit body here so the fixture remains readable in test traces.
    World::default()
}

#[given("pagination documentation parameters without a limit")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn pagination_documentation_parameters_without_a_limit(world: &World) {
    let params = PageParams::new(None, None).expect("default params should be valid");
    world.page_params.set(params);
}

#[given("pagination documentation parameters with limit {limit:u64}")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn pagination_documentation_parameters_with_limit(world: &World, limit: u64) {
    let requested_limit = usize::try_from(limit).expect("fixture limit should fit usize");
    let params = PageParams::new(None, Some(requested_limit)).expect("params should be valid");
    world.page_params.set(params);
}

#[given("an invalid base64 cursor token {token}")]
fn an_invalid_base64_cursor_token(world: &World, token: String) { world.cursor_token.set(token); }

#[given("a base64url token containing invalid JSON")]
fn a_base64url_token_containing_invalid_json(world: &World) {
    let invalid_json = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"{not-json");
    world.cursor_token.set(invalid_json);
}

#[given("an oversized cursor token")]
fn an_oversized_cursor_token(world: &World) {
    world.cursor_token.set("a".repeat(OVERSIZED_CURSOR_LEN));
}

#[given("pagination errors of different documented variants")]
fn pagination_errors_of_different_documented_variants(world: &World) {
    let mut errors = Vec::new();
    collect_cursor_error(&mut errors, Cursor::<FixtureKey>::decode("not!valid"));

    let invalid_json = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"{not-json");
    collect_cursor_error(&mut errors, Cursor::<FixtureKey>::decode(&invalid_json));
    collect_cursor_error(
        &mut errors,
        Cursor::<FixtureKey>::decode(&"a".repeat(OVERSIZED_CURSOR_LEN)),
    );

    errors.push(CursorError::Serialize {
        message: "fixture serialization failed".to_owned(),
    });

    world.cursor_errors.set(errors);
}

fn collect_cursor_error(
    errors: &mut Vec<CursorError>,
    result: Result<Cursor<FixtureKey>, CursorError>,
) {
    if let Err(error) = result {
        errors.push(error);
    }
}

#[when("the documentation cursor is decoded")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn the_documentation_cursor_is_decoded(world: &World) {
    let token = world
        .cursor_token
        .get()
        .expect("cursor token should be set")
        .clone();
    world
        .decode_result
        .set(Cursor::<FixtureKey>::decode(&token));
}

#[when("pagination documentation parameters are created with limit {limit:u64}")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn pagination_documentation_parameters_are_created_with_limit(world: &World, limit: u64) {
    let requested_limit = usize::try_from(limit).expect("fixture limit should fit usize");
    let result = PageParams::new(None, Some(requested_limit));
    world.page_params_result.set(result);
}

#[then("the documented normalized limit equals DEFAULT_LIMIT")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn the_documented_normalized_limit_equals_default_limit(world: &World) {
    let params = world.page_params.get().expect("page params should be set");

    assert_eq!(params.limit(), DEFAULT_LIMIT);
}

#[then("the documented normalized limit equals MAX_LIMIT")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn the_documented_normalized_limit_equals_max_limit(world: &World) {
    let params = world.page_params.get().expect("page params should be set");

    assert_eq!(params.limit(), MAX_LIMIT);
}

#[then("page parameter creation fails with InvalidLimit error")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn page_parameter_creation_fails_with_invalid_limit_error(world: &World) {
    let result = world
        .page_params_result
        .get()
        .expect("page params result should be set");

    assert_eq!(result, Err(PageParamsError::InvalidLimit));
}

#[then("decoding fails with InvalidBase64 error")]
fn decoding_fails_with_invalid_base64_error(world: &World) {
    assert_decode_error(world, |error| {
        matches!(error, CursorError::InvalidBase64 { .. })
    });
}

#[then("decoding fails with Deserialize error")]
fn decoding_fails_with_deserialize_error(world: &World) {
    assert_decode_error(world, |error| {
        matches!(error, CursorError::Deserialize { .. })
    });
}

#[then("decoding fails with TokenTooLong error")]
fn decoding_fails_with_token_too_long_error(world: &World) {
    assert_decode_error(world, |error| {
        matches!(error, CursorError::TokenTooLong { .. })
    });
}

#[expect(
    clippy::expect_used,
    reason = "BDD helpers use expect for clear failures"
)]
fn assert_decode_error(world: &World, matches_error: impl FnOnce(&CursorError) -> bool) {
    let result = world
        .decode_result
        .get()
        .expect("decode result should be set");
    let error = result.as_ref().expect_err("cursor decoding should fail");

    assert!(matches_error(error));
}

#[then("each pagination error display string contains a descriptive message")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn each_pagination_error_display_string_contains_a_descriptive_message(world: &World) {
    let errors = world
        .cursor_errors
        .get()
        .expect("cursor errors should be set");

    for error in errors {
        let display = format!("{error}");
        assert!(
            has_descriptive_error_text(&display),
            "error display string should be descriptive; got: {display}"
        );
    }
}

fn has_descriptive_error_text(display: &str) -> bool {
    display.contains("base64")
        || display.contains("deserialization")
        || display.contains("exceeds maximum length")
        || display.contains("serialization")
}

#[scenario(path = "tests/features/pagination_documentation.feature")]
fn pagination_documentation_invariants(world: World) { drop(world); }
