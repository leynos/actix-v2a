//! Cache-header helpers for live SSE event streams.

use crate::sse::header::SseHeader;

/// HTTP header name for cache-control directives.
pub const CACHE_CONTROL_HEADER: &str = "Cache-Control";

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
/// use actix_v2a::{
///     CACHE_CONTROL_HEADER,
///     EVENT_STREAM_CACHE_CONTROL,
///     apply_event_stream_cache_control,
/// };
///
/// let mut headers = Vec::new();
/// apply_event_stream_cache_control(&mut headers);
///
/// let cache_header = headers.first().expect("cache header should be set");
/// assert_eq!(
///     (cache_header.name(), cache_header.value()),
///     (CACHE_CONTROL_HEADER, EVENT_STREAM_CACHE_CONTROL)
/// );
/// ```
pub fn apply_event_stream_cache_control(headers: &mut Vec<SseHeader>) {
    headers.retain(|header| !header.has_name(CACHE_CONTROL_HEADER));
    headers.push(SseHeader::new(
        CACHE_CONTROL_HEADER,
        EVENT_STREAM_CACHE_CONTROL,
    ));
}

#[cfg(test)]
mod tests {
    //! Regression coverage for event-stream cache-control helpers.

    use proptest::prelude::*;

    use super::{
        CACHE_CONTROL_HEADER,
        EVENT_STREAM_CACHE_CONTROL,
        apply_event_stream_cache_control,
    };
    use crate::SseHeader;

    /// Generate SSE header strategies for cache-control idempotence tests.
    ///
    /// The strategy mixes arbitrary header names with common `Cache-Control`
    /// casing variants and arbitrary string values, so the property test can
    /// prove canonical cache-control replacement without losing unrelated
    /// headers.
    fn arbitrary_sse_header() -> impl Strategy<Value = SseHeader> {
        let arbitrary_name = any::<String>();
        let cache_control_name = prop_oneof![
            Just(CACHE_CONTROL_HEADER.to_owned()),
            Just("cache-control".to_owned()),
            Just("CACHE-CONTROL".to_owned()),
            Just("Cache-control".to_owned()),
        ];

        (
            prop_oneof![arbitrary_name, cache_control_name],
            any::<String>(),
        )
            .prop_map(|(name, value)| SseHeader::new(name, value))
    }

    #[test]
    fn apply_event_stream_cache_control_sets_expected_value() {
        let mut headers = Vec::new();

        apply_event_stream_cache_control(&mut headers);

        let header = headers.first().expect("cache header should be present");
        assert_eq!(header.name(), CACHE_CONTROL_HEADER);
        assert_eq!(header.value(), EVENT_STREAM_CACHE_CONTROL);
    }

    #[test]
    fn apply_event_stream_cache_control_replaces_existing_cache_policy() {
        let mut headers = vec![SseHeader::new(CACHE_CONTROL_HEADER, "public, max-age=60")];

        apply_event_stream_cache_control(&mut headers);

        assert_eq!(headers.len(), 1);
        assert_eq!(
            headers
                .first()
                .expect("cache header should be present")
                .value(),
            EVENT_STREAM_CACHE_CONTROL
        );
    }

    #[test]
    fn apply_event_stream_cache_control_preserves_unrelated_headers() {
        let mut headers = vec![SseHeader::new("Content-Type", "text/event-stream")];

        apply_event_stream_cache_control(&mut headers);

        assert!(headers.contains(&SseHeader::new("Content-Type", "text/event-stream")));
    }

    #[test]
    fn apply_event_stream_cache_control_is_deterministic_when_repeated() {
        let mut headers = Vec::new();

        apply_event_stream_cache_control(&mut headers);
        apply_event_stream_cache_control(&mut headers);

        let values: Vec<_> = headers
            .iter()
            .filter(|header| header.has_name(CACHE_CONTROL_HEADER))
            .collect();
        assert_eq!(values.len(), 1);
        assert_eq!(
            values
                .first()
                .expect("cache header should be present")
                .value(),
            EVENT_STREAM_CACHE_CONTROL
        );
    }

    proptest! {
        #[test]
        fn apply_event_stream_cache_control_is_idempotent_over_arbitrary_headers(
            mut headers in prop::collection::vec(arbitrary_sse_header(), 0..20),
        ) {
            let unrelated_headers: Vec<_> = headers
                .iter()
                .filter(|header| !header.has_name(CACHE_CONTROL_HEADER))
                .cloned()
                .collect();

            apply_event_stream_cache_control(&mut headers);
            apply_event_stream_cache_control(&mut headers);

            let cache_headers: Vec<_> = headers
                .iter()
                .filter(|header| header.has_name(CACHE_CONTROL_HEADER))
                .collect();
            prop_assert_eq!(cache_headers.len(), 1);

            if let Some(cache_header) = cache_headers.first() {
                prop_assert_eq!(cache_header.name(), CACHE_CONTROL_HEADER);
                prop_assert_eq!(cache_header.value(), EVENT_STREAM_CACHE_CONTROL);
            }

            let retained_unrelated_headers: Vec<_> = headers
                .iter()
                .filter(|header| !header.has_name(CACHE_CONTROL_HEADER))
                .cloned()
                .collect();
            prop_assert_eq!(retained_unrelated_headers, unrelated_headers);
        }
    }
}
