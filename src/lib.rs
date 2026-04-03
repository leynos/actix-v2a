//! Shared version 2a Actix components and HTTP primitives.
//!
//! This crate hosts reusable transport-facing building blocks extracted from
//! application code so Wildside, Corbusier, and other services can share one
//! stable set of contracts.

pub mod error;
pub mod http;
pub mod idempotency;
pub mod openapi;
pub mod pagination;
pub mod sse;

pub use error::{Error, ErrorCode, ErrorValidationError, TRACE_ID_HEADER};
pub use idempotency::{
    IDEMPOTENCY_CONFLICT_MESSAGE,
    IDEMPOTENCY_KEY_HEADER,
    IdempotencyKey,
    IdempotencyKeyValidationError,
    IdempotencyLookupQuery,
    IdempotencyLookupResult,
    IdempotencyRecord,
    MutationType,
    MutationTypeValidationError,
    PayloadHash,
    PayloadHashError,
    ReplayMetadata,
    ResponseSnapshot,
    canonicalize_and_hash,
    extract_idempotency_key,
    map_idempotency_key_error,
};
pub use openapi::{ErrorCodeSchema, ErrorSchema, ReplayMetadataSchema};
pub use pagination::{
    Cursor,
    CursorError,
    DEFAULT_LIMIT,
    Direction,
    MAX_LIMIT,
    PageParams,
    PageParamsError,
    Paginated,
    PaginationLinks,
};
pub use sse::{
    EventId,
    EventIdValidationError,
    LAST_EVENT_ID_HEADER,
    ReplayCursor,
    extract_replay_cursor,
    map_replay_cursor_error,
};
