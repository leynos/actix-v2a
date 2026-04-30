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
use rstest_bdd::{Slot, StepResult};
use rstest_bdd_macros::{ScenarioState, given, scenario, then, when};
use serde::{Deserialize, Serialize, Serializer};

const OVERSIZED_CURSOR_LEN: usize = 8 * 1024 + 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FixtureKey {
    created_at: String,
    id: String,
}

struct FailingKey;

impl Serialize for FailingKey {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Err(serde::ser::Error::custom("fixture serialization failed"))
    }
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
fn pagination_documentation_parameters_without_a_limit(world: &World) -> StepResult<(), String> {
    let params = PageParams::new(None, None)
        .map_err(|error| format!("default params should be valid: {error}"))?;
    world.page_params.set(params);
    Ok(())
}

#[given("pagination documentation parameters with limit {limit:u64}")]
fn pagination_documentation_parameters_with_limit(
    world: &World,
    limit: u64,
) -> StepResult<(), String> {
    let requested_limit = usize::try_from(limit)
        .map_err(|error| format!("fixture limit should fit usize: {error}"))?;
    let params = PageParams::new(None, Some(requested_limit))
        .map_err(|error| format!("params should be valid: {error}"))?;
    world.page_params.set(params);
    Ok(())
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

    collect_cursor_error(&mut errors, Cursor::new(FailingKey).encode());

    world.cursor_errors.set(errors);
}

fn collect_cursor_error(errors: &mut Vec<CursorError>, result: Result<impl Sized, CursorError>) {
    if let Err(error) = result {
        errors.push(error);
    }
}

#[when("the documentation cursor is decoded")]
fn the_documentation_cursor_is_decoded(world: &World) -> StepResult<(), String> {
    let token = world
        .cursor_token
        .get()
        .ok_or_else(|| "cursor token should be set".to_owned())?
        .clone();
    world
        .decode_result
        .set(Cursor::<FixtureKey>::decode(&token));
    Ok(())
}

#[when("pagination documentation parameters are created with limit {limit:u64}")]
fn pagination_documentation_parameters_are_created_with_limit(
    world: &World,
    limit: u64,
) -> StepResult<(), String> {
    let requested_limit = usize::try_from(limit)
        .map_err(|error| format!("fixture limit should fit usize: {error}"))?;
    let result = PageParams::new(None, Some(requested_limit));
    world.page_params_result.set(result);
    Ok(())
}

#[then("the documented normalized limit equals DEFAULT_LIMIT")]
fn the_documented_normalized_limit_equals_default_limit(world: &World) -> StepResult<(), String> {
    assert_normalized_limit(world, DEFAULT_LIMIT)
}

#[then("the documented normalized limit equals MAX_LIMIT")]
fn the_documented_normalized_limit_equals_max_limit(world: &World) -> StepResult<(), String> {
    assert_normalized_limit(world, MAX_LIMIT)
}

fn assert_normalized_limit(world: &World, expected: usize) -> StepResult<(), String> {
    let params = world
        .page_params
        .get()
        .ok_or_else(|| "page params should be set".to_owned())?;

    if params.limit() == expected {
        Ok(())
    } else {
        Err(format!(
            "expected normalised limit {expected}, got {}",
            params.limit()
        ))
    }
}

#[then("page parameter creation fails with InvalidLimit error")]
fn page_parameter_creation_fails_with_invalid_limit_error(world: &World) -> StepResult<(), String> {
    let result = world
        .page_params_result
        .get()
        .ok_or_else(|| "page params result should be set".to_owned())?;

    if result == Err(PageParamsError::InvalidLimit) {
        Ok(())
    } else {
        Err(format!("expected InvalidLimit error, got {result:?}"))
    }
}

#[then("decoding fails with InvalidBase64 error")]
fn decoding_fails_with_invalid_base64_error(world: &World) -> StepResult<(), String> {
    assert_decode_error(world, |error| {
        matches!(error, CursorError::InvalidBase64 { .. })
    })
}

#[then("decoding fails with Deserialize error")]
fn decoding_fails_with_deserialize_error(world: &World) -> StepResult<(), String> {
    assert_decode_error(world, |error| {
        matches!(error, CursorError::Deserialize { .. })
    })
}

#[then("decoding fails with TokenTooLong error")]
fn decoding_fails_with_token_too_long_error(world: &World) -> StepResult<(), String> {
    assert_decode_error(world, |error| {
        matches!(error, CursorError::TokenTooLong { .. })
    })
}

fn assert_decode_error(
    world: &World,
    matches_error: impl FnOnce(&CursorError) -> bool,
) -> StepResult<(), String> {
    let result = world
        .decode_result
        .get()
        .ok_or_else(|| "decode result should be set".to_owned())?;

    match result {
        Err(error) if matches_error(&error) => Ok(()),
        Err(error) => Err(format!(
            "cursor decoding failed with unexpected error: {error:?}"
        )),
        Ok(cursor) => Err(format!("cursor decoding should fail, got {cursor:?}")),
    }
}

#[then("each documented cursor error variant is represented")]
fn each_documented_cursor_error_variant_is_represented(world: &World) -> StepResult<(), String> {
    let errors = world
        .cursor_errors
        .get()
        .ok_or_else(|| "cursor errors should be set".to_owned())?;

    let has_invalid_base64 = errors.iter().any(
        |error| matches!(error, CursorError::InvalidBase64 { message } if !message.is_empty()),
    );
    let has_deserialize = errors
        .iter()
        .any(|error| matches!(error, CursorError::Deserialize { message } if !message.is_empty()));
    let has_token_too_long = errors.iter().any(
        |error| matches!(error, CursorError::TokenTooLong { max_len } if *max_len == 8 * 1024),
    );
    let has_serialize = errors
        .iter()
        .any(|error| matches!(error, CursorError::Serialize { message } if !message.is_empty()));

    let all_variants_represented = [
        has_invalid_base64,
        has_deserialize,
        has_token_too_long,
        has_serialize,
    ]
    .into_iter()
    .all(std::convert::identity);

    if all_variants_represented {
        Ok(())
    } else {
        Err(format!(
            "expected every documented cursor error variant, got {errors:?}"
        ))
    }
}

#[test]
fn cursor_error_display_messages_match_snapshots() {
    insta::assert_snapshot!(
        "cursor_error_invalid_base64_display",
        CursorError::InvalidBase64 {
            message: "invalid byte 33, offset 3.".to_owned(),
        }
        .to_string()
    );
    insta::assert_snapshot!(
        "cursor_error_deserialize_display",
        CursorError::Deserialize {
            message: "expected ident at line 1 column 2".to_owned(),
        }
        .to_string()
    );
    insta::assert_snapshot!(
        "cursor_error_token_too_long_display",
        CursorError::TokenTooLong { max_len: 8 * 1024 }.to_string()
    );
    insta::assert_snapshot!(
        "cursor_error_serialize_display",
        CursorError::Serialize {
            message: "fixture serialization failed".to_owned(),
        }
        .to_string()
    );
}

#[test]
fn page_params_error_display_messages_match_snapshots() {
    insta::assert_snapshot!(
        "page_params_error_invalid_limit_display",
        PageParamsError::InvalidLimit.to_string()
    );
}

#[scenario(path = "tests/features/pagination_documentation.feature")]
fn pagination_documentation_invariants(world: World) { drop(world); }
