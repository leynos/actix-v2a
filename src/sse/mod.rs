//! Server-Sent Events (SSE) wire-level helpers.
//!
//! This module provides validated types for SSE event identifiers and replay
//! cursors, along with helper functions for extracting the `Last-Event-ID`
//! request header. These helpers ensure wire safety without imposing opinions
//! on identifier generation strategies or event store persistence models.

pub mod event_id;
pub mod replay_cursor;

pub use event_id::{EventId, EventIdValidationError};
pub use replay_cursor::{
    LAST_EVENT_ID_HEADER,
    ReplayCursor,
    ReplayCursorError,
    extract_replay_cursor,
    map_replay_cursor_error,
};
