//! Behavioural tests for the shared SSE wire contract.

use std::{
    fmt,
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
use tracing_subscriber::{Layer, prelude::*};

struct BufferLayer(Arc<Mutex<Vec<String>>>);

impl<S: tracing::Subscriber> Layer<S> for BufferLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        use tracing_subscriber::field::Visit;

        struct Collector(String);

        impl Visit for Collector {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
                use fmt::Write as _;

                let result = write!(self.0, " {}={:?}", field.name(), value);
                debug_assert!(result.is_ok(), "writing to String should not fail");
            }
        }

        let mut collector = Collector(String::new());
        event.record(&mut collector);

        if let Ok(mut buffer) = self.0.lock() {
            buffer.push(collector.0.trim_start().to_owned());
        }
    }
}

struct TracingGuard(Mutex<Option<tracing::subscriber::DefaultGuard>>);

impl fmt::Debug for TracingGuard {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TracingGuard(..)")
    }
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.0.lock() {
            let _ = guard.take();
        }
    }
}

#[derive(Debug, Default, ScenarioState)]
struct World {
    actix_headers: Slot<HeaderMap>,
    headers: Slot<Vec<SseHeader>>,
    replay_cursor_error: Slot<ReplayCursorError>,
    replay_cursor: Slot<Option<ReplayCursor>>,
    response_headers: Slot<Vec<SseHeader>>,
    heartbeat_frame: Slot<String>,
    event_frame: Slot<String>,
    log_buffer: Slot<Arc<Mutex<Vec<String>>>>,
    should_use_actix_path: Slot<bool>,
    stream_reset_frame: Slot<String>,
    tracing_guard: Slot<TracingGuard>,
}

impl World {
    fn install_tracing(&self) {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::registry().with(BufferLayer(Arc::clone(&buffer)));
        let guard = TracingGuard(Mutex::new(Some(tracing::subscriber::set_default(
            subscriber,
        ))));

        self.log_buffer.set(buffer);
        self.tracing_guard.set(guard);
    }
}

#[fixture]
fn world() -> World {
    // Keep the fixture explicit so scenario failures print a useful state type.
    let world = World::default();
    world.install_tracing();
    world
}

#[given("a reconnect request with Last-Event-ID {event_id}")]
fn a_reconnect_request_with_last_event_id(world: &World, event_id: String) {
    world
        .headers
        .set(vec![SseHeader::new(LAST_EVENT_ID_HEADER, event_id)]);
    world.should_use_actix_path.set(false);
}

#[given("a request with duplicate Last-Event-ID headers {first_id} and {second_id}")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn a_request_with_duplicate_last_event_id_headers(
    world: &World,
    first_id: String,
    second_id: String,
) {
    let mut headers = HeaderMap::new();
    let header_name = HeaderName::from_static("last-event-id");
    headers.append(
        header_name.clone(),
        HeaderValue::from_str(&first_id).expect("fixture header should be valid"),
    );
    headers.append(
        header_name,
        HeaderValue::from_str(&second_id).expect("fixture header should be valid"),
    );

    world.actix_headers.set(headers);
    world.should_use_actix_path.set(true);
}

#[given("a request with a Last-Event-ID header containing a forbidden character")]
#[expect(
    unsafe_code,
    reason = "BDD fixture needs an otherwise impossible Actix header value"
)]
fn a_request_with_a_last_event_id_header_containing_a_forbidden_character(world: &World) {
    let mut headers = HeaderMap::new();
    let header_name = HeaderName::from_static("last-event-id");
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
}

#[given("an Actix request with a non-UTF-8 Last-Event-ID header value")]
#[expect(
    unsafe_code,
    reason = "BDD fixture needs to construct invalid UTF-8 header value"
)]
fn an_actix_request_with_a_non_utf_8_last_event_id_header_value(world: &World) {
    let mut headers = HeaderMap::new();
    let header_name = HeaderName::from_static("last-event-id");
    let non_utf8_bytes = &[0xff, 0xfe, 0xfd];
    // SAFETY: This BDD fixture intentionally builds a non-UTF-8 header payload
    // to exercise `to_str()` failure. The byte slice is static test-only data
    // and the unsafe call is scoped to test input construction, so no aliasing
    // or ownership invariants are changed.
    let header_value = unsafe { HeaderValue::from_maybe_shared_unchecked(non_utf8_bytes) };
    headers.insert(header_name, header_value);

    world.actix_headers.set(headers);
    world.should_use_actix_path.set(true);
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

#[when("the replay cursor extraction fails")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn the_replay_cursor_extraction_fails(world: &World) {
    let should_use_actix_path = world
        .should_use_actix_path
        .get()
        .expect("extraction path should be set");
    let error = if should_use_actix_path {
        let headers = world
            .actix_headers
            .get()
            .expect("Actix request headers should be set");
        extract_actix_replay_cursor(&headers).expect_err("replay cursor extraction should fail")
    } else {
        let headers = world.headers.get().expect("request headers should be set");
        extract_replay_cursor(&headers).expect_err("replay cursor extraction should fail")
    };

    world.replay_cursor_error.set(error);
}

#[when("the Actix replay cursor extraction fails")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn the_actix_replay_cursor_extraction_fails(world: &World) {
    let headers = world
        .actix_headers
        .get()
        .expect("Actix request headers should be set");
    let error = extract_actix_replay_cursor(&headers)
        .expect_err("Actix replay cursor extraction should fail");

    world.replay_cursor_error.set(error);
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

#[then("the shared Last-Event-ID header name ignores non-matching headers")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn the_shared_last_event_id_header_name_ignores_non_matching_headers() {
    let wrong_name = format!("{LAST_EVENT_ID_HEADER}-wrong");
    let headers = vec![SseHeader::new(&wrong_name, "evt-123")];
    let result =
        extract_replay_cursor(&headers).expect("header with wrong name should not be an error");

    assert!(
        result.is_none(),
        "extract_replay_cursor should return None when header name does not match \
         LAST_EVENT_ID_HEADER"
    );
}

#[then(
    "a tracing error is emitted with header_name {header_name} and error_variant {error_variant}"
)]
#[expect(
    clippy::expect_used,
    reason = "BDD steps use expect for clear failures"
)]
fn a_tracing_error_is_emitted_with_header_name_and_error_variant(
    world: &World,
    header_name: String,
    error_variant: String,
) {
    world
        .replay_cursor_error
        .get()
        .expect("replay cursor error should be set");

    assert!(traced_scenario_logs_contain(
        world,
        &format!("header_name=\"{header_name}\"")
    ));
    assert!(traced_scenario_logs_contain(
        world,
        &format!("error_variant=\"{error_variant}\"")
    ));
}

fn traced_scenario_logs_contain(world: &World, value: &str) -> bool {
    let Some(buffer) = world.log_buffer.get() else {
        return false;
    };

    buffer
        .lock()
        .is_ok_and(|lines| lines.iter().any(|line| line.contains(value)))
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
    let policy = HeartbeatPolicy::new(DEFAULT_HEARTBEAT_INTERVAL)
        .expect("DEFAULT_HEARTBEAT_INTERVAL should be a valid heartbeat interval");

    assert_eq!(
        policy.interval(),
        Duration::from_secs(20),
        "DEFAULT_HEARTBEAT_INTERVAL should encode a 20-second heartbeat interval"
    );
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
