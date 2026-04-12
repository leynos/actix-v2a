//! Server-Sent Events (SSE) wire-level helpers.
//!
//! This module provides validated types for SSE event identifiers and replay
//! cursors, along with helper functions for extracting the `Last-Event-ID`
//! request header, rendering SSE frames, and applying live-stream cache
//! headers. These helpers ensure wire safety without imposing opinions on
//! identifier generation strategies or event store persistence models.

pub mod cache_control;
pub mod event_id;
pub mod frame;
pub mod heartbeat;
pub mod replay_cursor;
pub mod stream_reset;

pub use cache_control::{EVENT_STREAM_CACHE_CONTROL, apply_event_stream_cache_control};
pub use event_id::{EventId, EventIdValidationError};
pub use frame::{SseFrameError, render_comment_frame, render_event_frame};
pub use heartbeat::{
    DEFAULT_HEARTBEAT_INTERVAL,
    HeartbeatPolicy,
    HeartbeatPolicyError,
    render_heartbeat_frame,
};
pub use replay_cursor::{
    LAST_EVENT_ID_HEADER,
    ReplayCursor,
    ReplayCursorError,
    extract_replay_cursor,
    map_replay_cursor_error,
};
pub use stream_reset::{
    STREAM_RESET_EVENT_NAME,
    STREAM_RESET_REPLAY_UNAVAILABLE_PAYLOAD,
    render_stream_reset_frame,
};
