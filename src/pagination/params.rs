//! Query parameter parsing and normalization for paginated endpoints.
//!
//! [`PageParams`] is the shared query DTO for `cursor` and `limit`. Its custom
//! `Deserialize` implementation applies the same normalization as
//! [`PageParams::new`]: missing limits default to [`DEFAULT_LIMIT`], oversized
//! limits clamp to [`MAX_LIMIT`], and zero limits are rejected. This keeps
//! Actix Web query extractors and hand-built parameters aligned.

use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

/// Shared default page size for endpoints using the pagination foundation.
pub const DEFAULT_LIMIT: usize = 20;
/// Shared maximum page size for endpoints using the pagination foundation.
pub const MAX_LIMIT: usize = 100;
/// Shared query parameter name for page size.
pub const PAGE_PARAM_LIMIT: &str = "limit";
/// Shared query parameter name for opaque cursors.
pub const PAGE_PARAM_CURSOR: &str = "cursor";

/// Normalized pagination parameters.
///
/// `PageParams` is designed for direct use with query extractors. It applies
/// the shared default limit of 20, caps larger limits at 100, and rejects
/// zero-sized pages.
///
/// # Example
///
/// ```
/// use actix_v2a::pagination::PageParams;
///
/// let params = PageParams::new(Some("opaque-token".to_owned()), Some(150)).expect("valid params");
///
/// assert_eq!(params.limit(), 100);
/// assert_eq!(params.cursor(), Some("opaque-token"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PageParams {
    cursor: Option<String>,
    limit: usize,
}

/// Errors raised while normalizing page parameters.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PageParamsError {
    /// The requested limit was zero and cannot represent a page.
    #[error("page limit must be greater than zero")]
    InvalidLimit,
}

#[derive(Debug, Deserialize)]
struct RawPageParams {
    cursor: Option<String>,
    limit: Option<usize>,
}

impl PageParams {
    /// Construct normalized pagination parameters.
    ///
    /// # Errors
    ///
    /// Returns [`PageParamsError::InvalidLimit`] when `limit` is explicitly
    /// set to zero.
    pub fn new(cursor: Option<String>, limit: Option<usize>) -> Result<Self, PageParamsError> {
        let normalized_limit = normalize_limit(limit)?;
        Ok(Self {
            cursor,
            limit: normalized_limit,
        })
    }

    /// Borrow the opaque cursor token, if present.
    #[must_use]
    pub fn cursor(&self) -> Option<&str> { self.cursor.as_deref() }

    /// Return the normalized page size.
    #[must_use]
    pub const fn limit(&self) -> usize { self.limit }
}

impl<'de> Deserialize<'de> for PageParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawPageParams::deserialize(deserializer)?;
        Self::new(raw.cursor, raw.limit).map_err(serde::de::Error::custom)
    }
}

fn normalize_limit(limit: Option<usize>) -> Result<usize, PageParamsError> {
    match limit {
        None => Ok(DEFAULT_LIMIT),
        Some(0) => Err(PageParamsError::InvalidLimit),
        Some(value) => Ok(value.min(MAX_LIMIT)),
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for page parameter normalization.

    use rstest::rstest;
    use serde_json::json;

    use super::{DEFAULT_LIMIT, MAX_LIMIT, PageParams, PageParamsError};

    #[test]
    fn page_params_default_limit_to_shared_default() {
        let params = PageParams::new(None, None).expect("default params should be valid");

        assert_eq!(params.limit(), DEFAULT_LIMIT);
        assert_eq!(params.cursor(), None);
    }

    #[rstest]
    #[case::one_below_max(MAX_LIMIT - 1, MAX_LIMIT - 1)]
    #[case::exact_max(MAX_LIMIT, MAX_LIMIT)]
    #[case::one_above_max(MAX_LIMIT + 1, MAX_LIMIT)]
    #[case::far_above_max(MAX_LIMIT + 50, MAX_LIMIT)]
    fn page_params_normalize_limits_around_shared_maximum(
        #[case] requested_limit: usize,
        #[case] expected_limit: usize,
    ) {
        let params = PageParams::new(Some("opaque".to_owned()), Some(requested_limit))
            .expect("limit should normalize");

        assert_eq!(params.limit(), expected_limit);
        assert_eq!(params.cursor(), Some("opaque"));
    }

    #[test]
    fn page_params_reject_zero_limit() {
        let result = PageParams::new(None, Some(0));

        assert_eq!(result, Err(PageParamsError::InvalidLimit));
    }

    #[test]
    fn page_params_deserialization_normalizes_limit() {
        let params: PageParams = serde_json::from_value(json!({
            "cursor": "opaque",
            "limit": 999
        }))
        .expect("deserialization should succeed");

        assert_eq!(params.limit(), MAX_LIMIT);
        assert_eq!(params.cursor(), Some("opaque"));
    }
}
