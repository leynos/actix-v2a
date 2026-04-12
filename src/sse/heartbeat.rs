//! Shared heartbeat policy and frame helpers for SSE streams.

use std::time::Duration;

use thiserror::Error;

use crate::sse::{SseFrameError, frame::render_comment_frame};

/// Default interval for SSE heartbeat traffic.
pub const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(20);

/// Errors returned when constructing a [`HeartbeatPolicy`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum HeartbeatPolicyError {
    /// The caller supplied a zero-length heartbeat interval.
    #[error("heartbeat interval must be greater than zero")]
    ZeroInterval,
}

/// Typed heartbeat policy for SSE connections.
///
/// This type holds interval data only. It does not own timer scheduling or
/// stream lifecycle management, which remain the application's responsibility.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use actix_v2a::{DEFAULT_HEARTBEAT_INTERVAL, HeartbeatPolicy};
///
/// let default_policy = HeartbeatPolicy::default();
/// assert_eq!(default_policy.interval(), DEFAULT_HEARTBEAT_INTERVAL);
///
/// let custom_policy = HeartbeatPolicy::new(Duration::from_secs(5))
///     .expect("custom heartbeat interval should validate");
/// assert_eq!(custom_policy.interval(), Duration::from_secs(5));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeartbeatPolicy {
    interval: Duration,
}

impl HeartbeatPolicy {
    /// Construct a heartbeat policy with an explicit interval override.
    ///
    /// # Errors
    ///
    /// Returns [`HeartbeatPolicyError::ZeroInterval`] when `interval` is
    /// [`Duration::ZERO`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// use actix_v2a::HeartbeatPolicy;
    ///
    /// let policy =
    ///     HeartbeatPolicy::new(Duration::from_secs(15)).expect("custom interval should validate");
    ///
    /// assert_eq!(policy.interval(), Duration::from_secs(15));
    /// ```
    #[must_use = "ignoring heartbeat policy construction can hide an invalid interval override"]
    pub const fn new(interval: Duration) -> Result<Self, HeartbeatPolicyError> {
        if interval.is_zero() {
            return Err(HeartbeatPolicyError::ZeroInterval);
        }

        Ok(Self { interval })
    }

    /// Access the configured heartbeat interval.
    #[must_use]
    pub const fn interval(&self) -> Duration { self.interval }
}

impl Default for HeartbeatPolicy {
    fn default() -> Self {
        Self {
            interval: DEFAULT_HEARTBEAT_INTERVAL,
        }
    }
}

/// Render the canonical heartbeat frame for an SSE stream.
///
/// The frame is the empty SSE comment `:\n\n`, which keeps idle connections
/// warm without inventing a domain event.
///
/// # Errors
///
/// Propagates [`SseFrameError`] from [`render_comment_frame`]. The shared
/// heartbeat frame is currently infallible because it renders an empty
/// comment.
///
/// # Examples
///
/// ```
/// use actix_v2a::render_heartbeat_frame;
///
/// let frame = render_heartbeat_frame().expect("heartbeat frame should render");
///
/// assert_eq!(frame, ":\n\n");
/// ```
#[must_use = "heartbeat frames must be written to the response stream to keep the connection warm"]
pub fn render_heartbeat_frame() -> Result<String, SseFrameError> { render_comment_frame("") }

#[cfg(test)]
mod tests {
    //! Regression coverage for the shared SSE heartbeat helpers.

    use std::time::Duration;

    use rstest::rstest;

    use super::{
        DEFAULT_HEARTBEAT_INTERVAL,
        HeartbeatPolicy,
        HeartbeatPolicyError,
        render_heartbeat_frame,
    };

    #[test]
    fn default_heartbeat_policy_uses_adr_interval() {
        let policy = HeartbeatPolicy::default();

        assert_eq!(policy.interval(), DEFAULT_HEARTBEAT_INTERVAL);
        assert_eq!(policy.interval(), Duration::from_secs(20));
    }

    #[rstest]
    #[case(Duration::from_secs(1))]
    #[case(Duration::from_secs(5))]
    #[case(Duration::from_millis(250))]
    fn heartbeat_policy_preserves_explicit_override(#[case] interval: Duration) {
        let policy =
            HeartbeatPolicy::new(interval).expect("non-zero heartbeat interval should validate");

        assert_eq!(policy.interval(), interval);
    }

    #[test]
    fn heartbeat_policy_rejects_zero_duration_override() {
        let error =
            HeartbeatPolicy::new(Duration::ZERO).expect_err("zero heartbeat interval should fail");

        assert_eq!(error, HeartbeatPolicyError::ZeroInterval);
    }

    #[test]
    fn render_heartbeat_frame_emits_canonical_empty_comment() {
        let frame = render_heartbeat_frame().expect("heartbeat frame should render");

        assert_eq!(frame, ":\n\n");
    }

    #[test]
    fn render_heartbeat_frame_is_deterministic_when_repeated() {
        let first_frame = render_heartbeat_frame().expect("heartbeat frame should render");
        let second_frame = render_heartbeat_frame().expect("heartbeat frame should render");

        assert_eq!(first_frame, second_frame);
    }
}
