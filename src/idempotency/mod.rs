//! Idempotency primitives for safe request retries.
//!
//! This module provides the reusable pieces applications need to implement an
//! idempotency store and consistent HTTP request handling without importing
//! application-specific repositories or service logic.

mod http;
mod key;
mod mutation_type;
mod payload;
mod record;

pub use http::{
    IDEMPOTENCY_CONFLICT_MESSAGE,
    IDEMPOTENCY_KEY_HEADER,
    extract_idempotency_key,
    map_idempotency_key_error,
};
pub use key::{IdempotencyKey, IdempotencyKeyValidationError};
pub use mutation_type::{MutationType, MutationTypeValidationError};
pub use payload::{PayloadHash, PayloadHashError, canonicalize_and_hash};
pub use record::{
    IdempotencyLookupQuery,
    IdempotencyLookupResult,
    IdempotencyRecord,
    ReplayMetadata,
    ResponseSnapshot,
};
