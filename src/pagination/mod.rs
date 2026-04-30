//! Shared opaque cursor and pagination envelope primitives.
//!
//! This module stays transport- and persistence-neutral so inbound HTTP
//! handlers and outbound repositories can share one cursor contract without
//! coupling the pagination model to Actix, Diesel, or endpoint-specific
//! schemata.
//!
//! # Cursor-based pagination
//!
//! Cursors are opaque tokens that encode a position within an ordered dataset.
//! They support bidirectional navigation via [`Direction`] (`Next` or `Prev`).
//!
//! # Ordering requirements
//!
//! Key types used with [`Cursor`] must satisfy strict ordering guarantees so
//! that pagination remains stable across all pages of one endpoint:
//!
//! - The key type must implement `Serialize` and `DeserializeOwned` so cursors can be encoded into
//!   and decoded from opaque base64url JSON tokens.
//! - The key fields must correspond to a total ordering in the backing store. A common pattern is
//!   ordering by a filterable field such as `created_at`, followed by a unique tie-breaker such as
//!   a UUID primary key.
//! - The ordering must remain consistent between requests for the same endpoint. If a query changes
//!   ordering between pages, clients may see skipped or duplicated records.
//!
//! Consumers are responsible for applying the same ordering and cursor
//! predicates in their repositories or query adapters. This module does not
//! validate database ordering at runtime because that guarantee depends on the
//! consumer's key type, filters, and indexes. Property-based tests for ordering
//! invariants therefore belong in the consuming application.
//!
//! # Default and maximum limits
//!
//! Page size limits are controlled by two shared constants:
//!
//! - [`DEFAULT_LIMIT`]: 20 records per page when no limit is supplied.
//! - [`MAX_LIMIT`]: 100 records per page as the shared upper bound.
//!
//! [`PageParams`] normalizes limits during construction and deserialization.
//! Missing limits use [`DEFAULT_LIMIT`], oversized limits clamp to
//! [`MAX_LIMIT`], and zero limits return [`PageParamsError::InvalidLimit`].
//!
//! ```
//! use actix_v2a::pagination::{DEFAULT_LIMIT, MAX_LIMIT, PageParams};
//!
//! let default_page = PageParams::new(None, None).expect("default page params are valid");
//! assert_eq!(default_page.limit(), DEFAULT_LIMIT);
//!
//! let capped_page = PageParams::new(None, Some(200)).expect("oversized limit should clamp");
//! assert_eq!(capped_page.limit(), MAX_LIMIT);
//!
//! let zero_limit = PageParams::new(None, Some(0));
//! assert_eq!(
//!     zero_limit,
//!     Err(actix_v2a::pagination::PageParamsError::InvalidLimit)
//! );
//! ```
//!
//! # Error mapping
//!
//! HTTP adapters should map pagination errors into the shared API error
//! envelope according to whether the caller or server caused the failure:
//!
//! | Error type          | Variant         | HTTP status | Error code                   |
//! |---------------------|-----------------|-------------|------------------------------|
//! | [`CursorError`]     | `InvalidBase64` | 400         | [`crate::ErrorCode::InvalidRequest`] |
//! | [`CursorError`]     | `Deserialize`   | 400         | [`crate::ErrorCode::InvalidRequest`] |
//! | [`CursorError`]     | `TokenTooLong`  | 400         | [`crate::ErrorCode::InvalidRequest`] |
//! | [`CursorError`]     | `Serialize`     | 500         | [`crate::ErrorCode::InternalError`]  |
//! | [`PageParamsError`] | `InvalidLimit`  | 400         | [`crate::ErrorCode::InvalidRequest`] |
//!
//! `CursorError::Serialize` is a server-side failure because the endpoint's own
//! key type could not be encoded. It should be logged and investigated rather
//! than reported as a malformed client cursor.
//!
//! For observability, [`Cursor::encode`] and [`Cursor::decode`] are
//! instrumented with `tracing` spans. The public [`Cursor::encode`] method also
//! emits a `tracing::error!` event before returning `CursorError::Serialize`,
//! so callers do not need to re-log that failure. Other [`CursorError`] variants and
//! [`PageParamsError`] variants are caller-controlled input failures; the
//! library does not emit error events for them.
//!
//! # Scope boundaries
//!
//! This module intentionally does not provide database filters, connection
//! pooling, endpoint-specific `OpenAPI` schemata, or framework extractors.
//! Inbound adapters can deserialize [`PageParams`] with framework facilities
//! such as Actix Web's query extractor, and outbound adapters must apply the
//! endpoint-specific ordering and cursor predicates.
//!
//! # Example
//!
//! ```
//! use actix_v2a::pagination::{Cursor, Direction, PageParams, Paginated, PaginationLinks};
//! use serde::{Deserialize, Serialize};
//! use url::Url;
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
//! struct UserKey {
//!     created_at: String,
//!     id: String,
//! }
//!
//! let params = PageParams::new(None, Some(25)).expect("valid page params");
//! let current_url = Url::parse("https://example.test/api/v1/users").expect("valid url");
//! let next_cursor = Cursor::with_direction(
//!     UserKey {
//!         created_at: "2026-03-22T10:30:00Z".to_owned(),
//!         id: "8b116c56-0a58-4c55-b7d7-06ee6bbddb8c".to_owned(),
//!     },
//!     Direction::Next,
//! )
//! .encode()
//! .expect("cursor encoding succeeds");
//!
//! let page = Paginated::new(
//!     vec!["Ada Lovelace"],
//!     params.limit(),
//!     PaginationLinks::from_request(&current_url, &params, Some(next_cursor.as_str()), None),
//! );
//!
//! assert_eq!(page.limit, 25);
//! assert!(page.links.next.is_some());
//! ```

mod cursor;
mod envelope;
mod params;

pub use cursor::{Cursor, CursorError, Direction};
pub use envelope::{Paginated, PaginationLinks};
pub use params::{
    DEFAULT_LIMIT,
    MAX_LIMIT,
    PAGE_PARAM_CURSOR,
    PAGE_PARAM_LIMIT,
    PageParams,
    PageParamsError,
};
