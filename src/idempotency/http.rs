//! HTTP contract helpers for idempotent requests.

use actix_web::http::header::HeaderMap;

use crate::{Error, IdempotencyKey, IdempotencyKeyValidationError};

/// HTTP header name for client-provided idempotency keys.
pub const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";

/// Shared client-facing message for payload conflicts on an existing key.
pub const IDEMPOTENCY_CONFLICT_MESSAGE: &str = "idempotency key reuse with different payload";

/// Extract the idempotency key from request headers.
///
/// Missing headers are allowed and return `Ok(None)`.
///
/// # Errors
///
/// Returns [`IdempotencyKeyValidationError`] when the header cannot be decoded
/// as UTF-8 or does not contain a valid UUID.
pub fn extract_idempotency_key(
    headers: &HeaderMap,
) -> Result<Option<IdempotencyKey>, IdempotencyKeyValidationError> {
    let Some(header_value) = headers.get(IDEMPOTENCY_KEY_HEADER) else {
        return Ok(None);
    };

    let header_text = header_value
        .to_str()
        .map_err(|_| IdempotencyKeyValidationError::InvalidKey)?;
    IdempotencyKey::new(header_text).map(Some)
}

/// Map header validation failures to the shared API error envelope.
#[must_use]
pub fn map_idempotency_key_error(error: &IdempotencyKeyValidationError) -> Error {
    match error {
        IdempotencyKeyValidationError::EmptyKey => {
            Error::invalid_request_static("idempotency-key header must not be empty")
        }
        IdempotencyKeyValidationError::InvalidKey => {
            Error::invalid_request_static("idempotency-key header must be a valid uuid")
        }
    }
}

#[cfg(test)]
mod tests {
    //! Regression coverage for the idempotency HTTP helpers.

    use actix_web::http::header::{HeaderMap, HeaderName, HeaderValue};

    use super::{extract_idempotency_key, map_idempotency_key_error};
    use crate::{ErrorCode, idempotency::http::IDEMPOTENCY_CONFLICT_MESSAGE};

    #[test]
    fn extract_idempotency_key_returns_none_when_header_missing() {
        let headers = HeaderMap::new();

        let key = extract_idempotency_key(&headers).expect("missing header should be allowed");

        assert_eq!(key, None);
    }

    #[test]
    fn extract_idempotency_key_parses_valid_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("idempotency-key"),
            HeaderValue::from_static("550e8400-e29b-41d4-a716-446655440000"),
        );

        let key = extract_idempotency_key(&headers)
            .expect("valid UUID header should parse")
            .expect("header should be present");

        assert_eq!(key.as_ref(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn map_idempotency_key_error_uses_invalid_request_envelope() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("idempotency-key"),
            HeaderValue::from_static("not-a-uuid"),
        );

        let error = extract_idempotency_key(&headers).expect_err("invalid UUID should fail");
        let mapped = map_idempotency_key_error(&error);

        assert_eq!(mapped.code(), ErrorCode::InvalidRequest);
        assert_eq!(
            mapped.message(),
            "idempotency-key header must be a valid uuid"
        );
    }

    #[test]
    fn conflict_message_is_stable() {
        assert_eq!(
            IDEMPOTENCY_CONFLICT_MESSAGE,
            "idempotency key reuse with different payload"
        );
    }
}
