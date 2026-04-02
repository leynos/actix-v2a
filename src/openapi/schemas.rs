//! `utoipa` schema fragments for shared envelopes.

use utoipa::ToSchema;

/// `OpenAPI` schema for [`crate::ErrorCode`].
#[derive(ToSchema)]
#[schema(as = crate::ErrorCode)]
pub enum ErrorCodeSchema {
    /// The request is malformed or fails validation.
    #[schema(rename = "invalid_request")]
    InvalidRequest,
    /// Authentication failed or is missing.
    #[schema(rename = "unauthorized")]
    Unauthorized,
    /// Authenticated but not permitted to perform this action.
    #[schema(rename = "forbidden")]
    Forbidden,
    /// The requested resource does not exist.
    #[schema(rename = "not_found")]
    NotFound,
    /// The request conflicts with current server state.
    #[schema(rename = "conflict")]
    Conflict,
    /// A dependent service is unavailable.
    #[schema(rename = "service_unavailable")]
    ServiceUnavailable,
    /// An unexpected error occurred on the server.
    #[schema(rename = "internal_error")]
    InternalError,
}

/// `OpenAPI` schema for [`crate::Error`].
#[derive(ToSchema)]
#[schema(as = crate::Error)]
#[expect(
    dead_code,
    reason = "Used only for OpenAPI schema generation via utoipa"
)]
pub struct ErrorSchema {
    /// Stable machine-readable error code.
    #[schema(example = "invalid_request")]
    code: ErrorCodeSchema,
    /// Human-readable message returned to clients.
    #[schema(example = "Something went wrong")]
    message: String,
    /// Correlation identifier for tracing this error across systems.
    #[schema(rename = "traceId", example = "trace-123")]
    trace_id: Option<String>,
    /// Supplementary error details for clients.
    details: Option<serde_json::Value>,
}

/// `OpenAPI` schema for [`crate::idempotency::ReplayMetadata`].
#[derive(ToSchema)]
#[schema(as = crate::idempotency::ReplayMetadata)]
#[expect(
    dead_code,
    reason = "Used only for OpenAPI schema generation via utoipa"
)]
pub struct ReplayMetadataSchema {
    /// Whether the response was replayed from an existing idempotency record.
    #[schema(example = true)]
    replayed: bool,
}

#[cfg(test)]
mod tests {
    //! Regression coverage for shared schema fragments.

    use utoipa::{PartialSchema, ToSchema};

    use super::{ErrorCodeSchema, ErrorSchema, ReplayMetadataSchema};

    fn schema_json<T: PartialSchema>() -> String {
        serde_json::to_string(&T::schema()).expect("schema should serialize")
    }

    #[test]
    fn error_code_schema_has_expected_name_and_variants() {
        let schema = schema_json::<ErrorCodeSchema>();

        assert_eq!(ErrorCodeSchema::name(), "crate.ErrorCode");
        for variant in [
            "invalid_request",
            "unauthorized",
            "forbidden",
            "not_found",
            "conflict",
            "service_unavailable",
            "internal_error",
        ] {
            assert!(schema.contains(variant), "missing variant {variant}");
        }
    }

    #[test]
    fn error_schema_has_expected_name_and_fields() {
        let schema = schema_json::<ErrorSchema>();

        assert_eq!(ErrorSchema::name(), "crate.Error");
        for field in ["code", "message", "traceId", "details"] {
            assert!(schema.contains(field), "missing field {field}");
        }
    }

    #[test]
    fn replay_metadata_schema_has_expected_name_and_field() {
        let schema = schema_json::<ReplayMetadataSchema>();

        assert_eq!(
            ReplayMetadataSchema::name(),
            "crate.idempotency.ReplayMetadata"
        );
        assert!(schema.contains("replayed"));
    }
}
