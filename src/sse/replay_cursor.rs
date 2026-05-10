//! Replay cursor and `Last-Event-ID` header extraction.

use std::fmt;

use thiserror::Error;

use crate::{
    Error,
    sse::{
        event_id::{EventId, EventIdValidationError},
        header::SseHeader,
    },
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

impl ReplayCursorError {
    pub(crate) const fn variant_name(&self) -> &'static str {
        match self {
            Self::InvalidHeader => "InvalidHeader",
            Self::Empty => "Empty",
            Self::ForbiddenCharacter => "ForbiddenCharacter",
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

fn log_replay_cursor_extraction_error(error: &ReplayCursorError, message: &'static str) {
    tracing::error!(
        error = %error,
        header_name = LAST_EVENT_ID_HEADER,
        error_variant = error.variant_name(),
        "{message}"
    );
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
/// Returns [`ReplayCursorError::InvalidHeader`] when duplicate
/// `Last-Event-ID` headers are present.
/// Returns [`ReplayCursorError::ForbiddenCharacter`] when the header
/// value contains carriage return (CR), line feed (LF), or NULL.
///
/// # Examples
///
/// ```
/// use actix_v2a::{SseHeader, extract_replay_cursor};
///
/// let headers = vec![SseHeader::new("last-event-id", "evt-123")];
///
/// let cursor = extract_replay_cursor(&headers)
///     .expect("valid header should parse")
///     .expect("header should be present");
///
/// assert_eq!(cursor.as_ref(), "evt-123");
/// ```
pub fn extract_replay_cursor(
    headers: &[SseHeader],
) -> Result<Option<ReplayCursor>, ReplayCursorError> {
    let mut header_values = headers
        .iter()
        .filter(|header| header.has_name(LAST_EVENT_ID_HEADER));
    let Some(header) = header_values.next() else {
        return Ok(None);
    };
    if header_values.next().is_some() {
        return Err(ReplayCursorError::InvalidHeader).inspect_err(|error| {
            log_replay_cursor_extraction_error(error, "replay cursor header extraction failed");
        });
    }

    let header_text = header.value();

    if header_text.is_empty() {
        return Ok(None);
    }

    EventId::new(header_text)
        .map(|id| Some(ReplayCursor::new(id)))
        .map_err(Into::into)
        .inspect_err(|error: &ReplayCursorError| {
            log_replay_cursor_extraction_error(error, "replay cursor header extraction failed");
        })
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

    use rstest::rstest;
    use tracing_test::traced_test;

    use super::{
        LAST_EVENT_ID_HEADER,
        ReplayCursor,
        ReplayCursorError,
        extract_replay_cursor,
        map_replay_cursor_error,
    };
    use crate::{
        ErrorCode,
        SseHeader,
        sse::event_id::{EventId, EventIdValidationError},
    };

    #[test]
    fn extract_replay_cursor_returns_none_when_header_missing() {
        let headers = Vec::new();

        let cursor = extract_replay_cursor(&headers).expect("missing header should be allowed");

        assert_eq!(cursor, None);
    }

    #[test]
    fn extract_replay_cursor_parses_valid_header() {
        let headers = vec![SseHeader::new("last-event-id", "evt-123")];

        let cursor = extract_replay_cursor(&headers)
            .expect("valid header should parse")
            .expect("header should be present");

        assert_eq!(cursor.event_id().as_str(), "evt-123");
    }

    #[test]
    fn extract_replay_cursor_returns_none_for_empty_header_value() {
        let headers = vec![SseHeader::new("last-event-id", "")];

        let cursor = extract_replay_cursor(&headers).expect("empty header should be allowed");

        assert_eq!(cursor, None);
    }

    #[test]
    #[traced_test]
    fn extract_replay_cursor_rejects_duplicate_headers() {
        let headers = vec![
            SseHeader::new("last-event-id", "evt-001"),
            SseHeader::new("last-event-id", "evt-002"),
        ];

        let error = extract_replay_cursor(&headers).expect_err("duplicate headers should fail");

        assert_eq!(error, ReplayCursorError::InvalidHeader);
        assert!(logs_contain("header_name=\"Last-Event-ID\""));
        assert!(logs_contain("error_variant=\"InvalidHeader\""));
        assert!(logs_contain("replay cursor header extraction failed"));
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
    #[traced_test]
    fn extract_replay_cursor_logs_forbidden_header_value() {
        let headers = vec![SseHeader::new("last-event-id", "evt\n123")];

        let error = extract_replay_cursor(&headers).expect_err("forbidden character should fail");

        assert_eq!(error, ReplayCursorError::ForbiddenCharacter);
        assert!(logs_contain("header_name=\"Last-Event-ID\""));
        assert!(logs_contain("error_variant=\"ForbiddenCharacter\""));
        assert!(logs_contain("replay cursor header extraction failed"));
    }

    #[test]
    fn extract_replay_cursor_preserves_leading_and_trailing_spaces() {
        let headers = vec![SseHeader::new("last-event-id", "  evt-001  ")];

        let cursor = extract_replay_cursor(&headers)
            .expect("header with spaces should parse")
            .expect("header should be present");

        assert_eq!(cursor.as_ref(), "  evt-001  ");
    }

    #[test]
    fn extract_replay_cursor_accepts_utf8_identifier() {
        let utf8_id = "événement-🎉-123";
        let headers = vec![SseHeader::new("last-event-id", utf8_id)];

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
