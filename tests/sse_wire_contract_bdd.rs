//! Behavioural tests for the shared SSE wire contract.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use actix_v2a::{
    CACHE_CONTROL_HEADER,
    DEFAULT_HEARTBEAT_INTERVAL,
    EVENT_STREAM_CACHE_CONTROL,
    EventId,
    HeartbeatPolicy,
    LAST_EVENT_ID_HEADER,
    ReplayCursor,
    ReplayCursorError,
    SseHeader,
    apply_event_stream_cache_control,
    extract_actix_replay_cursor,
    extract_replay_cursor,
    render_event_frame,
    render_heartbeat_frame,
    render_stream_reset_frame,
};
use actix_web::http::header::{HeaderMap, HeaderName, HeaderValue};
use rstest::fixture;
use rstest_bdd::Slot;
use rstest_bdd_macros::{ScenarioState, given, scenario, then, when};

#[path = "support/sse_trace_buffer.rs"]
mod sse_trace_buffer;

use sse_trace_buffer::LogBuffer;

type StepResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Debug, Default, ScenarioState)]
struct World {
    actix_headers: Slot<HeaderMap>,
    headers: Slot<Vec<SseHeader>>,
    replay_cursor_error: Slot<ReplayCursorError>,
    replay_cursor: Slot<Option<ReplayCursor>>,
    response_headers: Slot<Vec<SseHeader>>,
    heartbeat_frame: Slot<String>,
    event_frame: Slot<String>,
    log_buffer: Slot<LogBuffer>,
    should_use_actix_path: Slot<bool>,
    stream_reset_frame: Slot<String>,
}

#[fixture]
fn world() -> World {
    // Keep the fixture explicit so scenario failures print a useful state type.
    let world = World::default();
    world.log_buffer.set(Arc::new(Mutex::new(Vec::new())));
    world
}

#[given("a reconnect request with Last-Event-ID {event_id}")]
fn a_reconnect_request_with_last_event_id(world: &World, event_id: String) -> StepResult {
    EventId::new(event_id.clone())?;
    world
        .headers
        .set(vec![SseHeader::new(LAST_EVENT_ID_HEADER, event_id)]);
    world.should_use_actix_path.set(false);
    Ok(())
}

#[given("a request with duplicate Last-Event-ID headers {first_id} and {second_id}")]
fn a_request_with_duplicate_last_event_id_headers(
    world: &World,
    first_id: String,
    second_id: String,
) -> StepResult {
    let mut headers = HeaderMap::new();
    let header_name = HeaderName::from_static("last-event-id");
    headers.append(header_name.clone(), HeaderValue::from_str(&first_id)?);
    headers.append(header_name, HeaderValue::from_str(&second_id)?);

    world.actix_headers.set(headers);
    world.should_use_actix_path.set(true);
    Ok(())
}

#[given("a request with a Last-Event-ID header containing a forbidden character")]
#[expect(
    unsafe_code,
    reason = "BDD fixture needs an otherwise impossible Actix header value"
)]
fn a_request_with_a_last_event_id_header_containing_a_forbidden_character(
    world: &World,
) -> StepResult {
    let mut headers = HeaderMap::new();
    let header_name = HeaderName::from_bytes(LAST_EVENT_ID_HEADER.as_bytes())?;
    let forbidden_bytes = b"evt\n123";
    // SAFETY: This BDD fixture intentionally builds an invalid but UTF-8
    // Last-Event-ID payload so the adapter can exercise domain-level replay
    // cursor validation. The byte slice is static test data and the unsafe
    // call is scoped to test input construction, so no aliasing or ownership
    // invariants are changed.
    let header_value = unsafe { HeaderValue::from_maybe_shared_unchecked(forbidden_bytes) };
    headers.insert(header_name, header_value);

    world.actix_headers.set(headers);
    world.should_use_actix_path.set(true);
    Ok(())
}

#[given("an Actix request with a non-UTF-8 Last-Event-ID header value")]
#[expect(
    unsafe_code,
    reason = "BDD fixture needs to construct invalid UTF-8 header value"
)]
fn an_actix_request_with_a_non_utf_8_last_event_id_header_value(world: &World) -> StepResult {
    let mut headers = HeaderMap::new();
    let header_name = HeaderName::from_bytes(LAST_EVENT_ID_HEADER.as_bytes())?;
    let non_utf8_bytes = &[0xff, 0xfe, 0xfd];
    // SAFETY: This BDD fixture intentionally builds a non-UTF-8 header payload
    // to exercise `to_str()` failure. The byte slice is static test-only data
    // and the unsafe call is scoped to test input construction, so no aliasing
    // or ownership invariants are changed.
    let header_value = unsafe { HeaderValue::from_maybe_shared_unchecked(non_utf8_bytes) };
    headers.insert(header_name, header_value);

    world.actix_headers.set(headers);
    world.should_use_actix_path.set(true);
    Ok(())
}

#[given("an event-stream response")]
fn an_event_stream_response(world: &World) -> StepResult {
    HeaderValue::from_str(EVENT_STREAM_CACHE_CONTROL)?;
    world.response_headers.set(Vec::new());
    Ok(())
}

#[given("a downstream event with id {event_id}, event {event_name}, and payload {payload}")]
fn a_downstream_event_with_id_event_and_payload(
    world: &World,
    event_id: String,
    event_name: String,
    payload: String,
) -> StepResult {
    let id = EventId::new(event_id)?;
    let frame = render_event_frame(Some(&id), Some(&event_name), &payload)?;
    world.event_frame.set(frame);
    Ok(())
}

#[when("the replay cursor is extracted")]
fn the_replay_cursor_is_extracted(world: &World) -> StepResult {
    let headers = world.headers.get().ok_or("request headers should be set")?;
    let cursor = extract_replay_cursor(&headers)?;
    world.replay_cursor.set(cursor);
    Ok(())
}

#[when("the replay cursor extraction fails")]
fn the_replay_cursor_extraction_fails(world: &World) -> StepResult {
    let should_use_actix_path = world
        .should_use_actix_path
        .get()
        .ok_or("extraction path should be set")?;
    let error = if should_use_actix_path {
        let headers = world
            .actix_headers
            .get()
            .ok_or("Actix request headers should be set")?;
        sse_trace_buffer::with_tracing_buffer(&world.log_buffer, || {
            extract_actix_replay_cursor(&headers)
        })
        .err()
        .ok_or("replay cursor extraction should fail")?
    } else {
        let headers = world.headers.get().ok_or("request headers should be set")?;
        extract_replay_cursor(&headers)
            .err()
            .ok_or("replay cursor extraction should fail")?
    };

    world.replay_cursor_error.set(error);
    Ok(())
}

#[when("the Actix replay cursor extraction fails")]
fn the_actix_replay_cursor_extraction_fails(world: &World) -> StepResult {
    let headers = world
        .actix_headers
        .get()
        .ok_or("Actix request headers should be set")?;
    let error = sse_trace_buffer::with_tracing_buffer(&world.log_buffer, || {
        extract_actix_replay_cursor(&headers)
    })
    .err()
    .ok_or("Actix replay cursor extraction should fail")?;

    world.replay_cursor_error.set(error);
    Ok(())
}

#[when("the live-stream cache policy and heartbeat are applied")]
fn the_live_stream_cache_policy_and_heartbeat_are_applied(world: &World) -> StepResult {
    let mut headers = world
        .response_headers
        .get()
        .ok_or("response headers should be set")?
        .clone();
    apply_event_stream_cache_control(&mut headers);
    world.response_headers.set(headers);

    let heartbeat = render_heartbeat_frame()?;
    world.heartbeat_frame.set(heartbeat);
    Ok(())
}

#[when("the stream reset frame is rendered")]
fn the_stream_reset_frame_is_rendered(world: &World) -> StepResult {
    let frame = render_stream_reset_frame()?;
    world.stream_reset_frame.set(frame);
    Ok(())
}

#[then("the replay cursor preserves {event_id}")]
fn the_replay_cursor_preserves(world: &World, event_id: String) -> StepResult {
    let replay_cursor = world
        .replay_cursor
        .get()
        .ok_or("replay cursor slot should be set")?;
    let cursor = replay_cursor
        .as_ref()
        .ok_or("replay cursor should be present")?;

    assert_eq!(cursor.as_ref(), event_id);
    Ok(())
}

#[then("the shared Last-Event-ID header name ignores non-matching headers")]
fn the_shared_last_event_id_header_name_ignores_non_matching_headers() -> StepResult {
    let wrong_name = format!("{LAST_EVENT_ID_HEADER}-wrong");
    let headers = vec![SseHeader::new(&wrong_name, "evt-123")];
    let result = extract_replay_cursor(&headers)?;

    assert!(
        result.is_none(),
        "extract_replay_cursor should return None when header name does not match \
         LAST_EVENT_ID_HEADER"
    );
    Ok(())
}

#[then(
    "a tracing error is emitted with header_name {header_name} and error_variant {error_variant}"
)]
fn a_tracing_error_is_emitted_with_header_name_and_error_variant(
    world: &World,
    header_name: String,
    error_variant: String,
) -> StepResult {
    world
        .replay_cursor_error
        .get()
        .ok_or("replay cursor error should be set")?;

    assert!(traced_scenario_logs_contain(
        world,
        &format!("header_name=\"{header_name}\"")
    ));
    assert!(traced_scenario_logs_contain(
        world,
        &format!("error_variant=\"{error_variant}\"")
    ));
    Ok(())
}

fn traced_scenario_logs_contain(world: &World, value: &str) -> bool {
    sse_trace_buffer::logs_contain(&world.log_buffer, value)
}

#[then("the response uses the canonical no-store cache policy")]
fn the_response_uses_the_canonical_no_store_cache_policy(world: &World) -> StepResult {
    let headers = world
        .response_headers
        .get()
        .ok_or("response headers should be set")?;

    assert_eq!(
        headers
            .iter()
            .find(|header| header.name() == CACHE_CONTROL_HEADER)
            .ok_or("cache-control header should be present")?
            .value(),
        EVENT_STREAM_CACHE_CONTROL
    );
    Ok(())
}

#[then("the heartbeat frame is the canonical empty comment")]
fn the_heartbeat_frame_is_the_canonical_empty_comment(world: &World) -> StepResult {
    let heartbeat = world
        .heartbeat_frame
        .get()
        .ok_or("heartbeat frame should be set")?;

    assert_eq!(heartbeat, ":\n\n");
    let policy = HeartbeatPolicy::new(DEFAULT_HEARTBEAT_INTERVAL)?;

    assert_eq!(
        policy.interval(),
        Duration::from_secs(20),
        "DEFAULT_HEARTBEAT_INTERVAL should encode a 20-second heartbeat interval"
    );
    Ok(())
}

#[then("the event and stream reset frames match the approved wire format")]
fn the_event_and_stream_reset_frames_match_the_approved_wire_format(world: &World) -> StepResult {
    let event_frame = world.event_frame.get().ok_or("event frame should be set")?;
    let stream_reset_frame = world
        .stream_reset_frame
        .get()
        .ok_or("stream reset frame should be set")?;

    assert_eq!(
        event_frame,
        "id: evt-100\nevent: message_created\ndata: hello\n\n"
    );
    assert_eq!(
        stream_reset_frame,
        "event: stream_reset\ndata: {\"reason\":\"replay_unavailable\"}\n\n"
    );
    Ok(())
}

#[scenario(path = "tests/features/sse_wire_contract.feature")]
fn shared_sse_wire_contract(world: World) { drop(world); }
