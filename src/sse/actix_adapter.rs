//! Actix Web adapters for framework-agnostic SSE domain helpers.

use actix_web::http::header::{CACHE_CONTROL, HeaderMap, HeaderValue};

use crate::sse::{
    EVENT_STREAM_CACHE_CONTROL,
    LAST_EVENT_ID_HEADER,
    ReplayCursor,
    ReplayCursorError,
    SseHeader,
    extract_replay_cursor,
    replay_cursor::log_replay_cursor_extraction_error,
};

/// Apply the canonical SSE cache-control policy to Actix response headers.
pub fn apply_actix_event_stream_cache_control(headers: &mut HeaderMap) {
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static(EVENT_STREAM_CACHE_CONTROL),
    );
}

/// Extract a replay cursor from Actix request headers.
///
/// This adapter converts Actix's raw header values into UTF-8 domain headers
/// before delegating to the framework-agnostic replay cursor parser.
///
/// # Errors
///
/// Returns [`ReplayCursorError::InvalidHeader`] when the Actix header value is
/// not valid UTF-8, and otherwise returns the validation errors produced by
/// the domain-level replay cursor parser.
pub fn extract_actix_replay_cursor(
    headers: &HeaderMap,
) -> Result<Option<ReplayCursor>, ReplayCursorError> {
    let domain_headers = headers
        .get_all(crate::sse::LAST_EVENT_ID_HEADER)
        .map(|value| {
            value
                .to_str()
                .map(|text| SseHeader::new(LAST_EVENT_ID_HEADER, text))
                .map_err(|_| ReplayCursorError::InvalidHeader)
        })
        .collect::<Result<Vec<_>, _>>()
        .inspect_err(|error| {
            log_replay_cursor_extraction_error(error, "replay cursor header is invalid UTF-8");
        })?;

    extract_replay_cursor(&domain_headers).inspect_err(|error| {
        log_replay_cursor_extraction_error(error, "replay cursor header extraction failed");
    })
}

#[cfg(test)]
mod tests {
    //! Regression coverage for Actix SSE adapters.

    use std::sync::{
        Arc,
        Mutex,
        MutexGuard,
        atomic::{AtomicU64, Ordering},
    };

    use actix_web::http::header::{
        CACHE_CONTROL,
        CONTENT_TYPE,
        HeaderMap,
        HeaderName,
        HeaderValue,
    };
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

    use super::{apply_actix_event_stream_cache_control, extract_actix_replay_cursor};
    use crate::sse::{EVENT_STREAM_CACHE_CONTROL, ReplayCursorError};

    #[test]
    fn apply_actix_event_stream_cache_control_sets_expected_value() {
        let mut headers = HeaderMap::new();

        apply_actix_event_stream_cache_control(&mut headers);

        assert_eq!(
            headers
                .get(CACHE_CONTROL)
                .expect("cache header should be present"),
            EVENT_STREAM_CACHE_CONTROL
        );
    }

    #[test]
    fn apply_actix_event_stream_cache_control_replaces_existing_cache_policy() {
        let mut headers = HeaderMap::new();
        headers.insert(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=60"),
        );

        apply_actix_event_stream_cache_control(&mut headers);

        assert_eq!(
            headers
                .get(CACHE_CONTROL)
                .expect("cache header should be present"),
            EVENT_STREAM_CACHE_CONTROL
        );
    }

    #[test]
    fn apply_actix_event_stream_cache_control_preserves_unrelated_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/event-stream"));

        apply_actix_event_stream_cache_control(&mut headers);

        assert_eq!(
            headers
                .get(CONTENT_TYPE)
                .expect("content type should stay present"),
            "text/event-stream"
        );
    }

    #[test]
    fn extract_actix_replay_cursor_rejects_non_utf8_header_value() {
        let mut headers = HeaderMap::new();
        let header_name = HeaderName::from_static("last-event-id");
        let non_utf8_bytes = &[0xff, 0xfe, 0xfd];
        #[expect(
            unsafe_code,
            reason = "Test needs to construct invalid UTF-8 header value"
        )]
        // SAFETY: `non_utf8_bytes` is a local test-only byte slice. The
        // unsafe construction is intentionally used to build a non-UTF-8 header
        // value so `to_str()` can be exercised on invalid input. The call is
        // limited to this test setup and does not affect ownership or
        // aliasing guarantees elsewhere.
        let header_value = unsafe { HeaderValue::from_maybe_shared_unchecked(non_utf8_bytes) };
        headers.insert(header_name, header_value);

        let (error, error_log) = capture_error_log(|| {
            extract_actix_replay_cursor(&headers).expect_err("non-UTF-8 value should fail")
        });

        assert_eq!(error, ReplayCursorError::InvalidHeader);
        insta::with_settings!({ snapshot_path => "../../tests/snapshots" }, {
            insta::assert_snapshot!("actix_replay_cursor_non_utf8_error_log", error_log);
        });
    }

    fn capture_error_log<T>(operation: impl FnOnce() -> T) -> (T, String) {
        let recorder = TraceRecorder::default();
        let subscriber = RecordingSubscriber::new(recorder.clone());
        let result = with_default(subscriber, operation);

        (result, recorder.error_events().join("\n"))
    }

    #[derive(Clone, Default)]
    struct TraceRecorder {
        events: Arc<Mutex<Vec<RecordedEvent>>>,
    }

    impl TraceRecorder {
        fn push_event(&self, event: RecordedEvent) { recover_lock(&self.events).push(event); }

        fn error_events(&self) -> Vec<String> {
            recover_lock(&self.events)
                .iter()
                .filter(|event| event.level == Level::ERROR)
                .map(RecordedEvent::format_fields)
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

        fn new_span(&self, _attributes: &tracing::span::Attributes<'_>) -> Id {
            Id::from_u64(self.next_id.fetch_add(1, Ordering::Relaxed))
        }

        fn record(&self, _span: &Id, _values: &tracing::span::Record<'_>) {}

        fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

        fn event(&self, event: &Event<'_>) {
            let mut visitor = EventVisitor::default();
            event.record(&mut visitor);
            self.recorder.push_event(RecordedEvent {
                level: *event.metadata().level(),
                message: visitor.message,
                header_name: visitor.header_name,
                error_variant: visitor.error_variant,
            });
        }

        fn enter(&self, _span: &Id) {}

        fn exit(&self, _span: &Id) {}
    }

    #[derive(Clone)]
    struct RecordedEvent {
        level: Level,
        message: Option<String>,
        header_name: Option<String>,
        error_variant: Option<String>,
    }

    impl RecordedEvent {
        fn format_fields(&self) -> String {
            format!(
                "header_name={} error_variant={} message={}",
                self.header_name.as_deref().unwrap_or("<missing>"),
                self.error_variant.as_deref().unwrap_or("<missing>"),
                self.message.as_deref().unwrap_or("<missing>")
            )
        }
    }

    #[derive(Default)]
    struct EventVisitor {
        message: Option<String>,
        header_name: Option<String>,
        error_variant: Option<String>,
    }

    impl Visit for EventVisitor {
        fn record_str(&mut self, field: &Field, value: &str) {
            self.record_field(field, value.to_owned());
        }

        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.record_field(field, format!("{value:?}").trim_matches('"').to_owned());
        }
    }

    impl EventVisitor {
        fn record_field(&mut self, field: &Field, value: String) {
            match field.name() {
                "message" => self.message = Some(value),
                "header_name" => self.header_name = Some(value),
                "error_variant" => self.error_variant = Some(value),
                _ => {}
            }
        }
    }
}
