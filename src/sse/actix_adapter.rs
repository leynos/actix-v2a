//! Actix Web adapters for framework-agnostic SSE domain helpers.

use actix_web::http::header::{CACHE_CONTROL, HeaderMap, HeaderValue};

use crate::sse::{
    EVENT_STREAM_CACHE_CONTROL,
    ReplayCursor,
    ReplayCursorError,
    SseHeader,
    extract_replay_cursor,
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
                .map(|text| SseHeader::new(crate::sse::LAST_EVENT_ID_HEADER, text))
                .map_err(|_| ReplayCursorError::InvalidHeader)
        })
        .collect::<Result<Vec<_>, _>>()?;

    extract_replay_cursor(&domain_headers)
}

#[cfg(test)]
mod tests {
    //! Regression coverage for Actix SSE adapters.

    use actix_web::http::header::{
        CACHE_CONTROL,
        CONTENT_TYPE,
        HeaderMap,
        HeaderName,
        HeaderValue,
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
        let header_value = unsafe { HeaderValue::from_maybe_shared_unchecked(non_utf8_bytes) };
        headers.insert(header_name, header_value);

        let error = extract_actix_replay_cursor(&headers).expect_err("non-UTF-8 value should fail");

        assert_eq!(error, ReplayCursorError::InvalidHeader);
    }
}
