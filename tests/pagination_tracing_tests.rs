//! Tracing coverage for public pagination cursor operations.

use std::sync::{
    Arc,
    Mutex,
    MutexGuard,
    atomic::{AtomicU64, Ordering},
};

use actix_v2a::pagination::{Cursor, CursorError};
use base64::Engine as _;
use serde::{Serialize, Serializer};
use tracing::{
    Event,
    Id,
    Level,
    Metadata,
    Subscriber,
    field::{Field, Visit},
    level_filters::LevelFilter,
    subscriber::{Interest, with_default},
};

struct FailingKey;

impl Serialize for FailingKey {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Err(serde::ser::Error::custom("fixture serialization failed"))
    }
}

#[test]
fn successful_encode_produces_span_without_error_events() {
    let recorder = TraceRecorder::default();
    let subscriber = RecordingSubscriber::new(recorder.clone());
    let cursor = Cursor::new("key");

    let result = with_default(subscriber, || cursor.encode());

    assert_eq!(result, Ok("eyJrZXkiOiJrZXkiLCJkaXIiOiJOZXh0In0".to_owned()));
    assert!(recorder.span_names().contains(&"encode".to_owned()));
    assert!(recorder.error_events().is_empty());
}

#[test]
fn failing_encode_emits_one_serialization_error_event() {
    let recorder = TraceRecorder::default();
    let subscriber = RecordingSubscriber::new(recorder.clone());

    let result = with_default(subscriber, || Cursor::new(FailingKey).encode());

    assert_eq!(
        result,
        Err(CursorError::Serialize {
            message: "fixture serialization failed".to_owned(),
        })
    );
    let error_events = recorder.error_events();
    assert_eq!(error_events.len(), 1);
    assert!(
        error_events
            .iter()
            .any(|event| event.contains("cursor serialization failed"))
    );
}

#[test]
fn successful_decode_produces_span_without_error_events() {
    let cursor = Cursor::new("key");
    let token = cursor.encode().expect("cursor encoding should succeed");
    let recorder = TraceRecorder::default();
    let subscriber = RecordingSubscriber::new(recorder.clone());

    let result = with_default(subscriber, || Cursor::<String>::decode(&token));

    assert_eq!(result, Ok(Cursor::new("key".to_owned())));
    assert!(recorder.span_names().contains(&"decode".to_owned()));
    assert!(recorder.error_events().is_empty());
}

#[test]
fn failing_decode_produces_no_error_events() {
    let recorder = TraceRecorder::default();
    let subscriber = RecordingSubscriber::new(recorder.clone());
    let invalid_json = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"{not-json");

    with_default(subscriber, || {
        assert!(matches!(
            Cursor::<String>::decode("not!valid"),
            Err(CursorError::InvalidBase64 { .. })
        ));
        assert!(matches!(
            Cursor::<String>::decode(&"a".repeat(8 * 1024 + 1)),
            Err(CursorError::TokenTooLong { .. })
        ));
        assert!(matches!(
            Cursor::<serde_json::Value>::decode(&invalid_json),
            Err(CursorError::Deserialize { .. })
        ));
    });

    assert!(recorder.error_events().is_empty());
}

#[derive(Clone, Default)]
struct TraceRecorder {
    spans: Arc<Mutex<Vec<String>>>,
    events: Arc<Mutex<Vec<RecordedEvent>>>,
}

impl TraceRecorder {
    fn push_span(&self, name: &str) { recover_lock(&self.spans).push(name.to_owned()); }

    fn push_event(&self, event: RecordedEvent) { recover_lock(&self.events).push(event); }

    fn span_names(&self) -> Vec<String> { recover_lock(&self.spans).clone() }

    fn error_events(&self) -> Vec<String> {
        recover_lock(&self.events)
            .iter()
            .filter(|event| event.level == Level::ERROR)
            .filter_map(|event| event.message.clone())
            .collect()
    }
}

fn recover_lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

struct RecordingSubscriber {
    recorder: TraceRecorder,
    next_id: AtomicU64,
}

impl RecordingSubscriber {
    const fn new(recorder: TraceRecorder) -> Self {
        Self {
            recorder,
            next_id: AtomicU64::new(1),
        }
    }
}

impl Subscriber for RecordingSubscriber {
    fn register_callsite(&self, _metadata: &'static Metadata<'static>) -> Interest {
        Interest::always()
    }

    fn enabled(&self, _metadata: &Metadata<'_>) -> bool { true }

    fn max_level_hint(&self) -> Option<LevelFilter> { Some(LevelFilter::TRACE) }

    fn new_span(&self, attributes: &tracing::span::Attributes<'_>) -> Id {
        self.recorder.push_span(attributes.metadata().name());
        Id::from_u64(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    fn record(&self, _span: &Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        self.recorder.push_event(RecordedEvent {
            level: *event.metadata().level(),
            message: visitor.message,
        });
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

#[derive(Clone)]
struct RecordedEvent {
    level: Level,
    message: Option<String>,
}

#[derive(Default)]
struct MessageVisitor {
    message: Option<String>,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{value:?}"));
        }
    }
}
