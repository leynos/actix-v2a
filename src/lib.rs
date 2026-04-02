//! Shared version 2a Actix components and HTTP primitives.
//!
//! This crate hosts reusable transport-facing building blocks extracted from
//! application code so Wildside, Corbusier, and other services can share one
//! stable set of contracts.

pub mod pagination;

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
