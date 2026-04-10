//! Rendering helpers for SSE event and comment frames.

use std::borrow::Cow;

use thiserror::Error;

use crate::sse::EventId;

/// Errors returned when rendering SSE frames.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SseFrameError {
    /// The provided event name was empty.
    #[error("event name must not be empty when provided")]
    EmptyEventName,
    /// The provided event name would break the SSE wire format.
    #[error("event name must not contain carriage return, line feed, or null")]
    InvalidEventName,
    /// The event payload contained NULL, which cannot be represented safely.
    #[error("event data must not contain null")]
    InvalidData,
    /// The comment payload contained NULL, which cannot be represented safely.
    #[error("comment text must not contain null")]
    InvalidComment,
}

/// Render a complete SSE event frame.
///
/// The rendered frame always ends with a blank line. When `data` contains
/// `\n`, `\r`, or `\r\n`, each logical line is emitted as a separate `data:`
/// field so browsers reconstruct the original payload correctly. When
/// `event_name` is `None`, the browser-default `message` event semantics apply.
///
/// # Errors
///
/// Returns [`SseFrameError::EmptyEventName`] when `event_name` is `Some("")`,
/// [`SseFrameError::InvalidEventName`] when the name contains carriage return,
/// line feed, or NULL, and [`SseFrameError::InvalidData`] when `data`
/// contains NULL.
///
/// # Examples
///
/// ```
/// use actix_v2a::{EventId, render_event_frame};
///
/// let id = EventId::new("evt-001").expect("identifier should validate");
/// let frame = render_event_frame(Some(&id), Some("message_created"), "hello")
///     .expect("frame should render");
///
/// assert_eq!(
///     frame,
///     "id: evt-001\nevent: message_created\ndata: hello\n\n"
/// );
/// ```
pub fn render_event_frame(
    event_id: Option<&EventId>,
    event_name: Option<&str>,
    data: &str,
) -> Result<String, SseFrameError> {
    validate_data(data)?;
    validate_event_name(event_name)?;

    let mut frame = String::new();

    if let Some(identifier) = event_id {
        frame.push_str("id: ");
        frame.push_str(identifier.as_str());
        frame.push('\n');
    }

    if let Some(name) = event_name {
        frame.push_str("event: ");
        frame.push_str(name);
        frame.push('\n');
    }

    append_sse_lines(&mut frame, "data:", data);
    frame.push('\n');

    Ok(frame)
}

/// Render a complete SSE comment frame.
///
/// The rendered frame always ends with a blank line. Embedded `\n`, `\r`, and
/// `\r\n` sequences are treated as logical line breaks and rendered as
/// multiple comment lines. An empty comment renders as `:\n\n`, which is
/// suitable for heartbeat traffic.
///
/// # Errors
///
/// Returns [`SseFrameError::InvalidComment`] when `comment` contains NULL.
///
/// # Examples
///
/// ```
/// use actix_v2a::render_comment_frame;
///
/// let frame = render_comment_frame("keep-alive").expect("comment should render");
///
/// assert_eq!(frame, ": keep-alive\n\n");
/// ```
pub fn render_comment_frame(comment: &str) -> Result<String, SseFrameError> {
    validate_comment(comment)?;

    let mut frame = String::new();
    append_sse_lines(&mut frame, ":", comment);
    frame.push('\n');

    Ok(frame)
}

fn validate_event_name(event_name: Option<&str>) -> Result<(), SseFrameError> {
    let Some(name) = event_name else {
        return Ok(());
    };

    if name.is_empty() {
        return Err(SseFrameError::EmptyEventName);
    }

    if contains_wire_breaking_byte(name) {
        return Err(SseFrameError::InvalidEventName);
    }

    Ok(())
}

fn validate_data(data: &str) -> Result<(), SseFrameError> {
    if data.as_bytes().contains(&b'\0') {
        return Err(SseFrameError::InvalidData);
    }

    Ok(())
}

fn validate_comment(comment: &str) -> Result<(), SseFrameError> {
    if comment.as_bytes().contains(&b'\0') {
        return Err(SseFrameError::InvalidComment);
    }

    Ok(())
}

fn contains_wire_breaking_byte(value: &str) -> bool {
    value
        .as_bytes()
        .iter()
        .any(|byte| matches!(byte, b'\r' | b'\n' | b'\0'))
}

fn append_sse_lines(buffer: &mut String, prefix: &str, value: &str) {
    let normalized = normalize_newlines(value);
    for line in normalized.split('\n') {
        buffer.push_str(prefix);
        if line.is_empty() {
            buffer.push('\n');
        } else {
            buffer.push(' ');
            buffer.push_str(line);
            buffer.push('\n');
        }
    }
}

fn normalize_newlines(value: &str) -> Cow<'_, str> {
    if !value.contains('\r') {
        return Cow::Borrowed(value);
    }

    let mut normalized = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\r' {
            if chars.peek().copied() == Some('\n') {
                let _ = chars.next();
            }
            normalized.push('\n');
        } else {
            normalized.push(ch);
        }
    }

    Cow::Owned(normalized)
}

#[cfg(test)]
mod tests {
    //! Regression coverage for SSE frame rendering helpers.

    use rstest::rstest;

    use super::{SseFrameError, render_comment_frame, render_event_frame};
    use crate::EventId;

    #[test]
    fn render_event_frame_renders_id_event_and_single_line_data() {
        let id = EventId::new("evt-001").expect("identifier should validate");

        let frame = render_event_frame(Some(&id), Some("message_created"), "hello world")
            .expect("frame should render");

        assert_eq!(
            frame,
            "id: evt-001\nevent: message_created\ndata: hello world\n\n"
        );
    }

    #[test]
    fn render_event_frame_omits_optional_fields_when_not_supplied() {
        let frame = render_event_frame(None, None, "hello world").expect("frame should render");

        assert_eq!(frame, "data: hello world\n\n");
    }

    #[rstest]
    #[case("first\nsecond", "data: first\ndata: second\n\n")]
    #[case("first\rsecond", "data: first\ndata: second\n\n")]
    #[case("first\r\nsecond", "data: first\ndata: second\n\n")]
    #[case("first\n\nthird", "data: first\ndata:\ndata: third\n\n")]
    #[case("first\n", "data: first\ndata:\n\n")]
    #[case("", "data:\n\n")]
    fn render_event_frame_normalizes_logical_data_lines(
        #[case] data: &str,
        #[case] expected: &str,
    ) {
        let frame = render_event_frame(None, None, data).expect("frame should render");

        assert_eq!(frame, expected);
    }

    #[test]
    fn render_event_frame_accepts_unicode_event_name_and_payload() {
        let frame =
            render_event_frame(None, Some("événement"), "bonjour 🎉").expect("frame should render");

        assert_eq!(frame, "event: événement\ndata: bonjour 🎉\n\n");
    }

    #[test]
    fn render_comment_frame_renders_heartbeat_comment() {
        let frame = render_comment_frame("keep-alive").expect("comment should render");

        assert_eq!(frame, ": keep-alive\n\n");
    }

    #[rstest]
    #[case("first\nsecond", ": first\n: second\n\n")]
    #[case("first\rsecond", ": first\n: second\n\n")]
    #[case("first\r\nsecond", ": first\n: second\n\n")]
    #[case("first\n", ": first\n:\n\n")]
    #[case("", ":\n\n")]
    #[case("\n", ":\n:\n\n")]
    #[case("\r\n", ":\n:\n\n")]
    fn render_comment_frame_normalizes_logical_comment_lines(
        #[case] comment: &str,
        #[case] expected: &str,
    ) {
        let frame = render_comment_frame(comment).expect("comment should render");

        assert_eq!(frame, expected);
    }

    #[test]
    fn render_event_frame_rejects_empty_event_name() {
        let error =
            render_event_frame(None, Some(""), "hello world").expect_err("empty event should fail");

        assert_eq!(error, SseFrameError::EmptyEventName);
    }

    #[rstest]
    #[case("bad\nname")]
    #[case("bad\rname")]
    #[case("bad\0name")]
    fn render_event_frame_rejects_wire_breaking_event_name(#[case] event_name: &str) {
        let error = render_event_frame(None, Some(event_name), "hello world")
            .expect_err("wire-breaking event name should fail");

        assert_eq!(error, SseFrameError::InvalidEventName);
    }

    #[test]
    fn render_event_frame_rejects_null_in_data() {
        let error =
            render_event_frame(None, None, "bad\0data").expect_err("NULL in data should fail");

        assert_eq!(error, SseFrameError::InvalidData);
    }

    #[test]
    fn render_comment_frame_rejects_null_in_comment() {
        let error = render_comment_frame("bad\0comment").expect_err("NULL in comment should fail");

        assert_eq!(error, SseFrameError::InvalidComment);
    }
}
