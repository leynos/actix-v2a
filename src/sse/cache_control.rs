//! Cache-header helpers for live SSE event streams.

use actix_web::http::header::{CACHE_CONTROL, HeaderMap, HeaderValue};

/// Canonical `Cache-Control` policy for live event streams.
///
/// The directive combination disables reuse of long-lived event-stream
/// responses by browsers and intermediaries while staying within standard HTTP
/// cache-control semantics.
pub const EVENT_STREAM_CACHE_CONTROL: &str = "no-cache, no-store, must-revalidate";

/// Apply the canonical cache policy for a live SSE event stream.
///
/// Existing `Cache-Control` state is replaced deterministically. Unrelated
/// headers are left untouched.
///
/// # Examples
///
/// ```
/// use actix_v2a::{EVENT_STREAM_CACHE_CONTROL, apply_event_stream_cache_control};
/// use actix_web::http::header::{CACHE_CONTROL, HeaderMap};
///
/// let mut headers = HeaderMap::new();
/// apply_event_stream_cache_control(&mut headers);
///
/// assert_eq!(
///     headers
///         .get(CACHE_CONTROL)
///         .expect("cache header should be set"),
///     EVENT_STREAM_CACHE_CONTROL
/// );
/// ```
pub fn apply_event_stream_cache_control(headers: &mut HeaderMap) {
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static(EVENT_STREAM_CACHE_CONTROL),
    );
}

#[cfg(test)]
mod tests {
    //! Regression coverage for event-stream cache-control helpers.

    use actix_web::http::header::{CACHE_CONTROL, CONTENT_TYPE, HeaderMap, HeaderValue};

    use super::{EVENT_STREAM_CACHE_CONTROL, apply_event_stream_cache_control};

    #[test]
    fn apply_event_stream_cache_control_sets_expected_value() {
        let mut headers = HeaderMap::new();

        apply_event_stream_cache_control(&mut headers);

        assert_eq!(
            headers
                .get(CACHE_CONTROL)
                .expect("cache header should be present"),
            EVENT_STREAM_CACHE_CONTROL
        );
    }

    #[test]
    fn apply_event_stream_cache_control_replaces_existing_cache_policy() {
        let mut headers = HeaderMap::new();
        headers.insert(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=60"),
        );

        apply_event_stream_cache_control(&mut headers);

        assert_eq!(
            headers
                .get(CACHE_CONTROL)
                .expect("cache header should be present"),
            EVENT_STREAM_CACHE_CONTROL
        );
    }

    #[test]
    fn apply_event_stream_cache_control_preserves_unrelated_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/event-stream"));

        apply_event_stream_cache_control(&mut headers);

        assert_eq!(
            headers
                .get(CONTENT_TYPE)
                .expect("content type should stay present"),
            "text/event-stream"
        );
    }

    #[test]
    fn apply_event_stream_cache_control_is_deterministic_when_repeated() {
        let mut headers = HeaderMap::new();

        apply_event_stream_cache_control(&mut headers);
        apply_event_stream_cache_control(&mut headers);

        let values: Vec<_> = headers.get_all(CACHE_CONTROL).collect();
        assert_eq!(values.len(), 1);
        assert_eq!(
            values
                .first()
                .copied()
                .expect("a cache header should be present"),
            EVENT_STREAM_CACHE_CONTROL
        );
    }
}
