//! Replay cursor and `Last-Event-ID` header extraction.

use std::fmt;

use actix_web::http::header::HeaderMap;
use thiserror::Error;

use crate::{
    Error,
    sse::event_id::{EventId, EventIdValidationError},
};

/// HTTP header name for the SSE reconnection identifier.
pub const LAST_EVENT_ID_HEADER: &str = "Last-Event-ID";

/// Errors encountered when parsing the `Last-Event-ID` header.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReplayCursorError {
    /// The HTTP header was malformed (duplicate or non-UTF-8).
    #[error("last-event-id header is malformed")]
    InvalidHeader,
    /// The identifier value was empty.
    #[error("event identifier must not be empty")]
    Empty,
    /// The identifier value contained a forbidden character (CR, LF, or NULL).
    #[error("event identifier must not contain carriage return, line feed, or null")]
    ForbiddenCharacter,
}

impl From<EventIdValidationError> for ReplayCursorError {
    fn from(error: EventIdValidationError) -> Self {
        match error {
            EventIdValidationError::Empty => Self::Empty,
            EventIdValidationError::ForbiddenCharacter => Self::ForbiddenCharacter,
        }
    }
}

/// Replay cursor extracted from the `Last-Event-ID` request header.
///
/// This type wraps a validated [`EventId`] to distinguish between "an
/// identifier to attach to an outgoing SSE frame" and "an identifier received
/// from a client's reconnection header". The inner `EventId` carries the same
/// validation guarantees.
///
/// # Examples
///
/// ```
/// use actix_v2a::{EventId, ReplayCursor};
///
/// let id = EventId::new("evt-123").expect("valid identifier");
/// let cursor = ReplayCursor::new(id.clone());
///
/// assert_eq!(cursor.event_id(), &id);
/// assert_eq!(cursor.as_ref(), "evt-123");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReplayCursor(EventId);

impl ReplayCursor {
    /// Wrap an already validated event identifier.
    #[must_use]
    pub const fn new(event_id: EventId) -> Self { Self(event_id) }

    /// Access the wrapped event identifier.
    #[must_use]
    pub const fn event_id(&self) -> &EventId { &self.0 }

    /// Unwrap the inner event identifier.
    #[must_use]
    pub fn into_event_id(self) -> EventId { self.0 }
}

impl AsRef<str> for ReplayCursor {
    fn as_ref(&self) -> &str { self.0.as_ref() }
}

impl fmt::Display for ReplayCursor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(formatter) }
}

/// Extract the replay cursor from request headers.
///
/// Missing headers are allowed and return `Ok(None)`. An empty `Last-Event-ID`
/// header value is also treated as `Ok(None)`, consistent with the WHATWG
/// specification's treatment of an empty `id:` field as a reset of the last
/// event identifier.
///
/// # Errors
///
/// Returns [`ReplayCursorError::InvalidHeader`] when the header cannot be
/// decoded as UTF-8 or when duplicate `Last-Event-ID` headers are present.
/// Returns [`ReplayCursorError::ForbiddenCharacter`] when the header
/// value contains carriage return (CR), line feed (LF), or NULL.
///
/// # Examples
///
/// ```
/// use actix_v2a::extract_replay_cursor;
/// use actix_web::http::header::{HeaderMap, HeaderName, HeaderValue};
///
/// let mut headers = HeaderMap::new();
/// headers.insert(
///     HeaderName::from_static("last-event-id"),
///     HeaderValue::from_static("evt-123"),
/// );
///
/// let cursor = extract_replay_cursor(&headers)
///     .expect("valid header should parse")
///     .expect("header should be present");
///
/// assert_eq!(cursor.as_ref(), "evt-123");
/// ```
pub fn extract_replay_cursor(
    headers: &HeaderMap,
) -> Result<Option<ReplayCursor>, ReplayCursorError> {
    let mut header_values = headers.get_all(LAST_EVENT_ID_HEADER);
    let Some(header_value) = header_values.next() else {
        return Ok(None);
    };
    if header_values.next().is_some() {
        return Err(ReplayCursorError::InvalidHeader);
    }

    let header_text = std::str::from_utf8(header_value.as_bytes())
        .map_err(|_| ReplayCursorError::InvalidHeader)?;

    if header_text.is_empty() {
        return Ok(None);
    }

    EventId::new(header_text)
        .map(|id| Some(ReplayCursor::new(id)))
        .map_err(Into::into)
}

/// Map replay cursor validation failures to the shared API error envelope.
///
/// # Examples
///
/// ```
/// use actix_v2a::{ErrorCode, ReplayCursorError, map_replay_cursor_error};
///
/// let error = ReplayCursorError::InvalidHeader;
/// let api_error = map_replay_cursor_error(&error);
///
/// assert_eq!(api_error.code(), ErrorCode::InvalidRequest);
/// assert_eq!(api_error.message(), "last-event-id header is malformed");
/// ```
#[must_use]
pub fn map_replay_cursor_error(error: &ReplayCursorError) -> Error {
    match error {
        ReplayCursorError::Empty => {
            Error::invalid_request_static("last-event-id must not be empty")
        }
        ReplayCursorError::ForbiddenCharacter => Error::invalid_request_static(
            "last-event-id must not contain carriage return, line feed, or null",
        ),
        ReplayCursorError::InvalidHeader => {
            Error::invalid_request_static("last-event-id header is malformed")
        }
    }
}

#[cfg(test)]
mod tests {
    //! Regression coverage for the SSE replay cursor and header extraction.

    use actix_web::http::header::{HeaderMap, HeaderName, HeaderValue};
    use rstest::rstest;

    use super::{
        LAST_EVENT_ID_HEADER,
        ReplayCursor,
        ReplayCursorError,
        extract_replay_cursor,
        map_replay_cursor_error,
    };
    use crate::{
        ErrorCode,
        sse::event_id::{EventId, EventIdValidationError},
    };

    #[test]
    fn extract_replay_cursor_returns_none_when_header_missing() {
        let headers = HeaderMap::new();

        let cursor = extract_replay_cursor(&headers).expect("missing header should be allowed");

        assert_eq!(cursor, None);
    }

    #[test]
    fn extract_replay_cursor_parses_valid_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("last-event-id"),
            HeaderValue::from_static("evt-123"),
        );

        let cursor = extract_replay_cursor(&headers)
            .expect("valid header should parse")
            .expect("header should be present");

        assert_eq!(cursor.event_id().as_str(), "evt-123");
    }

    #[test]
    fn extract_replay_cursor_returns_none_for_empty_header_value() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("last-event-id"),
            HeaderValue::from_static(""),
        );

        let cursor = extract_replay_cursor(&headers).expect("empty header should be allowed");

        assert_eq!(cursor, None);
    }

    #[test]
    fn extract_replay_cursor_rejects_duplicate_headers() {
        let mut headers = HeaderMap::new();
        let header_name = HeaderName::from_static("last-event-id");
        headers.append(header_name.clone(), HeaderValue::from_static("evt-001"));
        headers.append(header_name, HeaderValue::from_static("evt-002"));

        let error = extract_replay_cursor(&headers).expect_err("duplicate headers should fail");

        assert_eq!(error, ReplayCursorError::InvalidHeader);
    }

    #[test]
    fn extract_replay_cursor_rejects_non_utf8_header_value() {
        let mut headers = HeaderMap::new();
        let header_name = HeaderName::from_static("last-event-id");
        let non_utf8_bytes = &[0xff, 0xfe, 0xfd];
        #[expect(
            unsafe_code,
            reason = "Test needs to construct invalid UTF-8 header value"
        )]
        let header_value = unsafe { HeaderValue::from_maybe_shared_unchecked(non_utf8_bytes) };
        headers.insert(header_name, header_value);

        let error = extract_replay_cursor(&headers).expect_err("non-UTF-8 value should fail");

        assert_eq!(error, ReplayCursorError::InvalidHeader);
    }

    #[rstest]
    #[case("evt\n123")]
    #[case("evt\r123")]
    #[case("evt\x00123")]
    fn event_id_validation_rejects_forbidden_characters(#[case] forbidden: &str) {
        // HeaderValue cannot contain CR, LF, or NULL per HTTP specification, so
        // these characters cannot appear in a valid Last-Event-ID header. The
        // EventId validation layer is tested separately to ensure these
        // characters are rejected if they somehow reach the validation logic.
        let result = EventId::new(forbidden);

        assert_eq!(result, Err(EventIdValidationError::ForbiddenCharacter));
    }

    #[test]
    fn extract_replay_cursor_preserves_leading_and_trailing_spaces() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("last-event-id"),
            HeaderValue::from_static("  evt-001  "),
        );

        let cursor = extract_replay_cursor(&headers)
            .expect("header with spaces should parse")
            .expect("header should be present");

        assert_eq!(cursor.as_ref(), "  evt-001  ");
    }

    #[test]
    fn extract_replay_cursor_accepts_utf8_identifier() {
        let mut headers = HeaderMap::new();
        let utf8_id = "événement-🎉-123";
        headers.insert(
            HeaderName::from_static("last-event-id"),
            HeaderValue::from_str(utf8_id).expect("UTF-8 should be valid header value"),
        );

        let cursor = extract_replay_cursor(&headers)
            .expect("UTF-8 header should parse")
            .expect("header should be present");

        assert_eq!(cursor.as_ref(), utf8_id);
    }

    #[test]
    fn replay_cursor_new_wraps_event_id() {
        let id = EventId::new("evt-test").expect("should validate");
        let cursor = ReplayCursor::new(id.clone());

        assert_eq!(cursor.event_id(), &id);
    }

    #[test]
    fn replay_cursor_into_event_id_unwraps() {
        let id = EventId::new("evt-unwrap").expect("should validate");
        let cursor = ReplayCursor::new(id.clone());

        assert_eq!(cursor.into_event_id(), id);
    }

    #[test]
    fn replay_cursor_as_ref_returns_identifier_string() {
        let id = EventId::new("evt-as-ref").expect("should validate");
        let cursor = ReplayCursor::new(id);

        assert_eq!(cursor.as_ref(), "evt-as-ref");
    }

    #[test]
    fn replay_cursor_display_matches_identifier() {
        let id = EventId::new("evt-display").expect("should validate");
        let cursor = ReplayCursor::new(id.clone());

        assert_eq!(cursor.to_string(), id.as_str());
    }

    #[rstest]
    #[case(ReplayCursorError::Empty, "last-event-id must not be empty")]
    #[case(
        ReplayCursorError::ForbiddenCharacter,
        "last-event-id must not contain carriage return, line feed, or null"
    )]
    #[case(ReplayCursorError::InvalidHeader, "last-event-id header is malformed")]
    fn map_replay_cursor_error_produces_invalid_request(
        #[case] error: ReplayCursorError,
        #[case] expected_message: &str,
    ) {
        let mapped = map_replay_cursor_error(&error);

        assert_eq!(mapped.code(), ErrorCode::InvalidRequest);
        assert_eq!(mapped.message(), expected_message);
    }

    #[test]
    fn last_event_id_header_constant_is_correct() {
        assert_eq!(LAST_EVENT_ID_HEADER, "Last-Event-ID");
    }
}
