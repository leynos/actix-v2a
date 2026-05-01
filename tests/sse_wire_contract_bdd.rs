//! Behavioural tests for the shared SSE wire contract.

use std::time::Duration;

use actix_v2a::{
    CACHE_CONTROL_HEADER,
    DEFAULT_HEARTBEAT_INTERVAL,
    EVENT_STREAM_CACHE_CONTROL,
    EventId,
    LAST_EVENT_ID_HEADER,
    ReplayCursor,
    SseHeader,
    apply_event_stream_cache_control,
    extract_replay_cursor,
    render_event_frame,
    render_heartbeat_frame,
    render_stream_reset_frame,
};
use rstest::fixture;
use rstest_bdd::Slot;
use rstest_bdd_macros::{ScenarioState, given, scenario, then, when};

#[derive(Debug, Default, ScenarioState)]
struct World {
    headers: Slot<Vec<SseHeader>>,
    replay_cursor: Slot<Option<ReplayCursor>>,
    response_headers: Slot<Vec<SseHeader>>,
    heartbeat_frame: Slot<String>,
    event_frame: Slot<String>,
    stream_reset_frame: Slot<String>,
}

#[fixture]
fn world() -> World {
    // Keep the fixture explicit so scenario failures print a useful state type.
    World::default()
}

#[given("a reconnect request with Last-Event-ID {event_id}")]
fn a_reconnect_request_with_last_event_id(world: &World, event_id: String) {
    world
        .headers
        .set(vec![SseHeader::new(LAST_EVENT_ID_HEADER, event_id)]);
}

#[given("an event-stream response")]
fn an_event_stream_response(world: &World) { world.response_headers.set(Vec::new()); }

#[given("a downstream event with id {event_id}, event {event_name}, and payload {payload}")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn a_downstream_event_with_id_event_and_payload(
    world: &World,
    event_id: String,
    event_name: String,
    payload: String,
) {
    let id = EventId::new(event_id).expect("fixture event id should be valid");
    let frame = render_event_frame(Some(&id), Some(&event_name), &payload)
        .expect("fixture event frame should render");
    world.event_frame.set(frame);
}

#[when("the replay cursor is extracted")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn the_replay_cursor_is_extracted(world: &World) {
    let headers = world.headers.get().expect("request headers should be set");
    let cursor = extract_replay_cursor(&headers).expect("valid reconnect header should parse");
    world.replay_cursor.set(cursor);
}

#[when("the live-stream cache policy and heartbeat are applied")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn the_live_stream_cache_policy_and_heartbeat_are_applied(world: &World) {
    let mut headers = world
        .response_headers
        .get()
        .expect("response headers should be set")
        .clone();
    apply_event_stream_cache_control(&mut headers);
    world.response_headers.set(headers);

    let heartbeat = render_heartbeat_frame().expect("heartbeat frame should render");
    world.heartbeat_frame.set(heartbeat);
}

#[when("the stream reset frame is rendered")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn the_stream_reset_frame_is_rendered(world: &World) {
    let frame = render_stream_reset_frame().expect("stream reset frame should render");
    world.stream_reset_frame.set(frame);
}

#[then("the replay cursor preserves {event_id}")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn the_replay_cursor_preserves(world: &World, event_id: String) {
    let replay_cursor = world
        .replay_cursor
        .get()
        .expect("replay cursor slot should be set");
    let cursor = replay_cursor
        .as_ref()
        .expect("replay cursor should be present");

    assert_eq!(cursor.as_ref(), event_id);
}

#[then("the shared Last-Event-ID header name is exported")]
fn the_shared_last_event_id_header_name_is_exported() {
    assert_eq!(LAST_EVENT_ID_HEADER, "Last-Event-ID");
}

#[then("the response uses the canonical no-store cache policy")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn the_response_uses_the_canonical_no_store_cache_policy(world: &World) {
    let headers = world
        .response_headers
        .get()
        .expect("response headers should be set");

    assert_eq!(
        headers
            .iter()
            .find(|header| header.name() == CACHE_CONTROL_HEADER)
            .expect("cache-control header should be present")
            .value(),
        EVENT_STREAM_CACHE_CONTROL
    );
}

#[then("the heartbeat frame is the canonical empty comment")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn the_heartbeat_frame_is_the_canonical_empty_comment(world: &World) {
    let heartbeat = world
        .heartbeat_frame
        .get()
        .expect("heartbeat frame should be set");

    assert_eq!(heartbeat, ":\n\n");
    assert_eq!(DEFAULT_HEARTBEAT_INTERVAL, Duration::from_secs(20));
}

#[then("the event and stream reset frames match the approved wire format")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn the_event_and_stream_reset_frames_match_the_approved_wire_format(world: &World) {
    let event_frame = world.event_frame.get().expect("event frame should be set");
    let stream_reset_frame = world
        .stream_reset_frame
        .get()
        .expect("stream reset frame should be set");

    assert_eq!(
        event_frame,
        "id: evt-100\nevent: message_created\ndata: hello\n\n"
    );
    assert_eq!(
        stream_reset_frame,
        "event: stream_reset\ndata: {\"reason\":\"replay_unavailable\"}\n\n"
    );
}

#[scenario(path = "tests/features/sse_wire_contract.feature")]
fn shared_sse_wire_contract(world: World) { drop(world); }
