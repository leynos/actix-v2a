//! Shared replay-reset helper for SSE streams.

use crate::sse::{SseFrameError, render_event_frame};

/// Canonical event name for replay-reset control messages.
pub const STREAM_RESET_EVENT_NAME: &str = "stream_reset";

/// Canonical payload for replay-reset control messages.
pub const STREAM_RESET_REPLAY_UNAVAILABLE_PAYLOAD: &str = r#"{"reason":"replay_unavailable"}"#;

/// Render the standard replay-unavailable `stream_reset` frame.
///
/// This helper is intentionally fixed to the shared control event defined by
/// ADR 001. It does not broaden the crate surface into a generic
/// application-event builder.
///
/// # Errors
///
/// Propagates [`SseFrameError`] from [`render_event_frame`]. The shared event
/// name and payload are currently valid and therefore render successfully.
///
/// # Examples
///
/// ```
/// use actix_v2a::render_stream_reset_frame;
///
/// let frame = render_stream_reset_frame().expect("stream reset frame should render");
///
/// assert_eq!(
///     frame,
///     "event: stream_reset\ndata: {\"reason\":\"replay_unavailable\"}\n\n"
/// );
/// ```
pub fn render_stream_reset_frame() -> Result<String, SseFrameError> {
    render_event_frame(
        None,
        Some(STREAM_RESET_EVENT_NAME),
        STREAM_RESET_REPLAY_UNAVAILABLE_PAYLOAD,
    )
}

#[cfg(test)]
mod tests {
    //! Regression coverage for the shared SSE replay-reset helper.

    use super::{
        STREAM_RESET_EVENT_NAME,
        STREAM_RESET_REPLAY_UNAVAILABLE_PAYLOAD,
        render_stream_reset_frame,
    };

    #[test]
    fn render_stream_reset_frame_emits_the_shared_control_event() {
        let frame = render_stream_reset_frame().expect("stream reset frame should render");

        assert_eq!(
            frame,
            "event: stream_reset\ndata: {\"reason\":\"replay_unavailable\"}\n\n"
        );
    }

    #[test]
    fn render_stream_reset_frame_is_deterministic_when_repeated() {
        let first_frame = render_stream_reset_frame().expect("stream reset frame should render");
        let second_frame = render_stream_reset_frame().expect("stream reset frame should render");

        assert_eq!(first_frame, second_frame);
    }

    #[test]
    fn stream_reset_constants_match_rendered_output() {
        let expected_frame = format!(
            "event: {STREAM_RESET_EVENT_NAME}\ndata: {STREAM_RESET_REPLAY_UNAVAILABLE_PAYLOAD}\n\n"
        );

        let frame = render_stream_reset_frame().expect("stream reset frame should render");

        assert_eq!(frame, expected_frame);
    }
}
