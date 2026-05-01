//! `utoipa` schema fragments for shared envelopes.

use serde::Serialize;
use utoipa::ToSchema;

/// `OpenAPI` schema for [`crate::ErrorCode`].
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
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

    use std::collections::BTreeSet;

    use utoipa::{PartialSchema, ToSchema};

    use super::{ErrorCodeSchema, ErrorSchema, ReplayMetadataSchema};
    use crate::ErrorCode;

    fn schema_json<T: PartialSchema>() -> Result<String, serde_json::Error> {
        serde_json::to_string(&T::schema())
    }

    #[test]
    fn error_code_schema_has_expected_name_and_variants() {
        let schema_json = schema_json::<ErrorCodeSchema>().expect("schema should serialize");
        let parsed_schema = serde_json::from_str::<serde_json::Value>(&schema_json)
            .expect("schema should parse as JSON");
        let variants = parsed_schema
            .get("enum")
            .and_then(serde_json::Value::as_array)
            .expect("schema should define enum variants");
        let actual_variants = variants
            .iter()
            .map(|variant| variant.as_str().expect("variant should be a string"))
            .collect::<BTreeSet<_>>();
        let expected_variants = BTreeSet::from([
            "invalid_request",
            "unauthorized",
            "forbidden",
            "not_found",
            "conflict",
            "service_unavailable",
            "internal_error",
        ]);

        assert_eq!(ErrorCodeSchema::name(), "crate.ErrorCode");
        assert_eq!(actual_variants, expected_variants);
    }

    #[test]
    fn error_code_schema_wire_names_match_error_code_wire_names() {
        let error_code_wire_names = [
            ErrorCode::InvalidRequest,
            ErrorCode::Unauthorized,
            ErrorCode::Forbidden,
            ErrorCode::NotFound,
            ErrorCode::Conflict,
            ErrorCode::ServiceUnavailable,
            ErrorCode::InternalError,
        ]
        .into_iter()
        .map(|error_code| serde_json::to_value(error_code).expect("error code should serialize"))
        .collect::<Vec<_>>();
        let schema_wire_names = [
            ErrorCodeSchema::InvalidRequest,
            ErrorCodeSchema::Unauthorized,
            ErrorCodeSchema::Forbidden,
            ErrorCodeSchema::NotFound,
            ErrorCodeSchema::Conflict,
            ErrorCodeSchema::ServiceUnavailable,
            ErrorCodeSchema::InternalError,
        ]
        .into_iter()
        .map(|error_code| serde_json::to_value(error_code).expect("schema code should serialize"))
        .collect::<Vec<_>>();

        assert_eq!(error_code_wire_names, schema_wire_names);
    }

    #[test]
    fn error_schema_has_expected_name_and_fields() {
        let schema_json = schema_json::<ErrorSchema>().expect("schema should serialize");
        let parsed_schema = serde_json::from_str::<serde_json::Value>(&schema_json)
            .expect("schema should parse as JSON");
        let properties = parsed_schema
            .get("properties")
            .and_then(serde_json::Value::as_object)
            .expect("schema should define object properties");
        let actual_fields = properties
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let expected_fields = BTreeSet::from(["code", "message", "traceId", "details"]);

        assert_eq!(ErrorSchema::name(), "crate.Error");
        assert_eq!(actual_fields, expected_fields);
    }

    #[test]
    fn replay_metadata_schema_has_expected_name_and_field() {
        let schema_json = schema_json::<ReplayMetadataSchema>().expect("schema should serialize");
        let parsed_schema = serde_json::from_str::<serde_json::Value>(&schema_json)
            .expect("schema should parse as JSON");
        let properties = parsed_schema
            .get("properties")
            .and_then(serde_json::Value::as_object)
            .expect("schema should define object properties");
        let actual_fields = properties
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let expected_fields = BTreeSet::from(["replayed"]);

        assert_eq!(
            ReplayMetadataSchema::name(),
            "crate.idempotency.ReplayMetadata"
        );
        assert_eq!(actual_fields, expected_fields);
    }
}
