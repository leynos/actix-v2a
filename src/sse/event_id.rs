//! Validated SSE event identifier for `id:` lines and replay cursors.

use std::fmt;

use thiserror::Error;

/// Validation errors for [`EventId`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EventIdValidationError {
    /// The identifier value was empty.
    #[error("event identifier must not be empty")]
    Empty,
    /// The identifier value contained a forbidden character (CR, LF, or NULL).
    #[error("event identifier must not contain carriage return, line feed, or null")]
    ForbiddenCharacter,
    /// The HTTP header was malformed (duplicate or non-UTF-8).
    #[error("last-event-id header is malformed")]
    InvalidHeader,
}

/// Validated SSE event identifier for `id:` lines and replay cursors.
///
/// The identifier is validated to ensure it does not contain carriage return
/// (U+000D), line feed (U+000A), or NULL (U+0000), as these characters would
/// corrupt the SSE wire format per the WHATWG HTML specification § 9.2.6.
///
/// The identifier is treated as an opaque string. No specific format (UUID,
/// integer, composite key) is imposed. Leading and trailing whitespace is
/// preserved; the identifier is not trimmed during validation.
///
/// # Examples
///
/// ```
/// use actix_v2a::EventId;
///
/// let id = EventId::new("evt-001").expect("simple identifier");
/// assert_eq!(id.as_str(), "evt-001");
///
/// let uuid_id = EventId::new("550e8400-e29b-41d4-a716-446655440000").expect("UUID identifier");
/// assert_eq!(uuid_id.as_str(), "550e8400-e29b-41d4-a716-446655440000");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventId(String);

impl EventId {
    /// Validate and construct an event identifier.
    ///
    /// # Errors
    ///
    /// Returns [`EventIdValidationError::Empty`] when the input is empty and
    /// [`EventIdValidationError::ForbiddenCharacter`] when the input contains
    /// carriage return (CR), line feed (LF), or NULL.
    ///
    /// # Examples
    ///
    /// ```
    /// use actix_v2a::EventId;
    ///
    /// let id = EventId::new("evt-123").expect("valid identifier");
    /// assert_eq!(id.as_str(), "evt-123");
    /// ```
    pub fn new(value: impl AsRef<str>) -> Result<Self, EventIdValidationError> {
        let value_str = value.as_ref();

        if value_str.is_empty() {
            return Err(EventIdValidationError::Empty);
        }

        // Scan for forbidden bytes: CR (0x0D), LF (0x0A), NULL (0x00).
        // Use byte iteration for efficiency since all three are single-byte
        // values in UTF-8.
        for byte in value_str.as_bytes() {
            if matches!(byte, b'\r' | b'\n' | b'\0') {
                return Err(EventIdValidationError::ForbiddenCharacter);
            }
        }

        Ok(Self(value_str.to_owned()))
    }

    /// Access the identifier as a string slice.
    #[must_use]
    pub const fn as_str(&self) -> &str { self.0.as_str() }
}

impl AsRef<str> for EventId {
    fn as_ref(&self) -> &str { self.as_str() }
}

impl fmt::Display for EventId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<EventId> for String {
    fn from(value: EventId) -> Self { value.0 }
}

impl TryFrom<String> for EventId {
    type Error = EventIdValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> { Self::new(value) }
}

#[cfg(test)]
mod tests {
    //! Regression coverage for SSE event identifier validation.

    use rstest::rstest;

    use super::{EventId, EventIdValidationError};

    #[test]
    fn new_accepts_simple_ascii_identifier() {
        let id = EventId::new("evt-001").expect("simple ASCII identifier should validate");

        assert_eq!(id.as_str(), "evt-001");
    }

    #[test]
    fn new_accepts_uuid_formatted_identifier() {
        let id = EventId::new("550e8400-e29b-41d4-a716-446655440000")
            .expect("UUID identifier should validate");

        assert_eq!(id.as_str(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn new_accepts_numeric_string_identifier() {
        let id = EventId::new("123456").expect("numeric identifier should validate");

        assert_eq!(id.as_str(), "123456");
    }

    #[test]
    fn new_accepts_identifier_with_embedded_spaces() {
        let id = EventId::new("event 123").expect("identifier with spaces should validate");

        assert_eq!(id.as_str(), "event 123");
    }

    #[test]
    fn new_accepts_identifier_with_leading_and_trailing_spaces() {
        let id = EventId::new("  evt-001  ")
            .expect("identifier with leading/trailing spaces should validate");

        assert_eq!(id.as_str(), "  evt-001  ");
    }

    #[test]
    fn new_accepts_non_ascii_unicode_identifier() {
        let id = EventId::new("événement-123").expect("accented characters should validate");

        assert_eq!(id.as_str(), "événement-123");
    }

    #[test]
    fn new_accepts_emoji_identifier() {
        let id = EventId::new("event-🎉-123").expect("emoji should validate");

        assert_eq!(id.as_str(), "event-🎉-123");
    }

    #[test]
    fn new_accepts_single_character_identifier() {
        let id = EventId::new("x").expect("single character should validate");

        assert_eq!(id.as_str(), "x");
    }

    #[test]
    fn as_str_returns_exact_input_string() {
        let input = "test-event-id";
        let id = EventId::new(input).expect("should validate");

        assert_eq!(id.as_str(), input);
    }

    #[test]
    fn display_matches_as_str() {
        let id = EventId::new("evt-display").expect("should validate");

        assert_eq!(id.to_string(), id.as_str());
    }

    #[test]
    fn from_event_id_for_string_roundtrips() {
        let original = "evt-roundtrip";
        let id = EventId::new(original).expect("should validate");
        let string_value: String = id.into();

        assert_eq!(string_value, original);
    }

    #[test]
    fn try_from_string_delegates_to_new() {
        let id = EventId::try_from("evt-123".to_owned()).expect("should validate");

        assert_eq!(id.as_str(), "evt-123");
    }

    #[test]
    fn new_rejects_empty_string() {
        let result = EventId::new("");

        assert_eq!(result, Err(EventIdValidationError::Empty));
    }

    #[rstest]
    #[case("\n")]
    #[case("evt\n123")]
    #[case("evt-123\n")]
    fn new_rejects_identifier_containing_line_feed(#[case] input: &str) {
        let result = EventId::new(input);

        assert_eq!(result, Err(EventIdValidationError::ForbiddenCharacter));
    }

    #[rstest]
    #[case("\r")]
    #[case("evt\r123")]
    #[case("evt-123\r")]
    fn new_rejects_identifier_containing_carriage_return(#[case] input: &str) {
        let result = EventId::new(input);

        assert_eq!(result, Err(EventIdValidationError::ForbiddenCharacter));
    }

    #[rstest]
    #[case("\0")]
    #[case("evt\x00123")]
    #[case("evt-123\0")]
    fn new_rejects_identifier_containing_null(#[case] input: &str) {
        let result = EventId::new(input);

        assert_eq!(result, Err(EventIdValidationError::ForbiddenCharacter));
    }

    #[test]
    fn new_rejects_string_containing_only_line_feed() {
        let result = EventId::new("\n");

        assert_eq!(result, Err(EventIdValidationError::ForbiddenCharacter));
    }
}
