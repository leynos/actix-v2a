//! Shared API error envelope types.
//!
//! The error payload in this module is transport-agnostic. Actix-specific
//! responder behaviour lives in [`crate::http::error`], which serializes this
//! type into HTTP responses and maps error codes to status codes.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error as ThisError;

/// HTTP header name used to propagate trace identifiers.
pub const TRACE_ID_HEADER: &str = "trace-id";

/// Stable machine-readable API error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The request is malformed or fails validation.
    InvalidRequest,
    /// Authentication failed or is missing.
    Unauthorized,
    /// Authenticated but not permitted to perform this action.
    Forbidden,
    /// The requested resource does not exist.
    NotFound,
    /// The request conflicts with current server state.
    Conflict,
    /// A dependent service is unavailable.
    ServiceUnavailable,
    /// An unexpected error occurred on the server.
    InternalError,
}

/// Validation errors for [`Error`].
#[derive(Debug, Clone, PartialEq, Eq, ThisError)]
pub enum ErrorValidationError {
    /// The message was empty once trimmed.
    #[error("error message must not be empty")]
    EmptyMessage,
    /// The trace identifier was empty once trimmed.
    #[error("trace identifier must not be empty")]
    EmptyTraceId,
}

/// API error response payload.
///
/// # Examples
///
/// ```
/// use actix_v2a::{Error, ErrorCode};
///
/// let error = Error::try_new(ErrorCode::NotFound, "missing").expect("valid payload");
///
/// assert_eq!(error.code(), ErrorCode::NotFound);
/// assert_eq!(error.message(), "missing");
/// ```
#[expect(
    clippy::error_impl_error,
    reason = "This crate intentionally exports a shared API error type named Error"
)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ErrorPayload")]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Error {
    code: ErrorCode,
    message: String,
    #[serde(alias = "trace_id", skip_serializing_if = "Option::is_none")]
    trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct ErrorPayload {
    code: ErrorCode,
    message: String,
    #[serde(alias = "trace_id")]
    trace_id: Option<String>,
    details: Option<Value>,
}

impl TryFrom<ErrorPayload> for Error {
    type Error = ErrorValidationError;

    fn try_from(payload: ErrorPayload) -> Result<Self, Self::Error> {
        let message = payload.message.trim().to_owned();
        let mut error = Self::try_new(payload.code, message)?;
        if let Some(trace_id) = payload.trace_id {
            error = error.try_with_trace_id(trace_id)?;
        }

        Ok(match payload.details {
            Some(details) => error.with_details(details),
            None => error,
        })
    }
}

impl Error {
    /// Construct a validated error payload.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorValidationError::EmptyMessage`] when `message` is blank
    /// once trimmed.
    pub fn try_new(
        code: ErrorCode,
        message: impl Into<String>,
    ) -> Result<Self, ErrorValidationError> {
        let message_text = message.into();
        if message_text.trim().is_empty() {
            return Err(ErrorValidationError::EmptyMessage);
        }

        Ok(Self {
            code,
            message: message_text,
            trace_id: None,
            details: None,
        })
    }

    /// Construct a library-owned error payload from a trusted static message.
    #[must_use]
    pub fn from_static(code: ErrorCode, message: &'static str) -> Self {
        Self {
            code,
            message: message.to_owned(),
            trace_id: None,
            details: None,
        }
    }

    /// Access the machine-readable error code.
    #[must_use]
    pub const fn code(&self) -> ErrorCode { self.code }

    /// Access the human-readable error message.
    #[must_use]
    pub const fn message(&self) -> &str { self.message.as_str() }

    /// Access the trace identifier, if present.
    #[must_use]
    pub fn trace_id(&self) -> Option<&str> { self.trace_id.as_deref() }

    /// Access structured error details, if present.
    #[must_use]
    pub const fn details(&self) -> Option<&Value> { self.details.as_ref() }

    /// Attach a validated trace identifier to the payload.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorValidationError::EmptyTraceId`] when `trace_id` is blank
    /// once trimmed.
    pub fn try_with_trace_id(
        mut self,
        trace_id: impl Into<String>,
    ) -> Result<Self, ErrorValidationError> {
        let trace_id_text = trace_id.into();
        let trimmed_trace_id = trace_id_text.trim();
        if trimmed_trace_id.is_empty() {
            return Err(ErrorValidationError::EmptyTraceId);
        }

        self.trace_id = Some(trimmed_trace_id.to_owned());
        Ok(self)
    }

    /// Attach structured details to the payload.
    #[must_use]
    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Construct an invalid-request error from a trusted static message.
    #[must_use]
    pub fn invalid_request_static(message: &'static str) -> Self {
        Self::from_static(ErrorCode::InvalidRequest, message)
    }

    /// Construct a conflict error from a trusted static message.
    #[must_use]
    pub fn conflict_static(message: &'static str) -> Self {
        Self::from_static(ErrorCode::Conflict, message)
    }

    /// Construct an internal error from a trusted static message.
    #[must_use]
    pub fn internal_static(message: &'static str) -> Self {
        Self::from_static(ErrorCode::InternalError, message)
    }

    /// Return the payload clients should see.
    ///
    /// Internal errors are redacted to a generic message and omit details.
    #[must_use]
    pub fn redacted(&self) -> Self {
        if matches!(self.code, ErrorCode::InternalError) {
            let mut redacted = Self::internal_static("Internal server error");
            redacted.trace_id.clone_from(&self.trace_id);
            redacted
        } else {
            self.clone()
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message())
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    //! Regression coverage for the shared error payload.

    use rstest::rstest;
    use serde_json::{Value, json};

    use super::{Error, ErrorCode, ErrorValidationError};

    #[derive(Debug)]
    struct RedactedExpectation {
        code: ErrorCode,
        message: &'static str,
        trace_id: Option<&'static str>,
        details: Option<Value>,
    }

    #[test]
    fn try_new_rejects_blank_messages() {
        let result = Error::try_new(ErrorCode::InvalidRequest, "   ");

        assert_eq!(result, Err(ErrorValidationError::EmptyMessage));
    }

    #[test]
    fn try_with_trace_id_rejects_blank_values() {
        let error = Error::invalid_request_static("bad request");

        let result = error.try_with_trace_id("   ");

        assert_eq!(result, Err(ErrorValidationError::EmptyTraceId));
    }

    #[rstest]
    #[case(
        Error::internal_static("boom")
            .try_with_trace_id("trace-123")
            .expect("trace identifier should be valid")
            .with_details(json!({"secret": true})),
        RedactedExpectation {
            code: ErrorCode::InternalError,
            message: "Internal server error",
            trace_id: Some("trace-123"),
            details: None
        }
    )]
    #[case(
        Error::invalid_request_static("bad request").with_details(json!({"field": "name"})),
        RedactedExpectation {
            code: ErrorCode::InvalidRequest,
            message: "bad request",
            trace_id: None,
            details: Some(json!({"field": "name"}))
        }
    )]
    fn redacted_behaviour(#[case] error: Error, #[case] expected: RedactedExpectation) {
        let redacted = error.redacted();

        assert_eq!(redacted.code(), expected.code);
        assert_eq!(redacted.message(), expected.message);
        assert_eq!(redacted.trace_id(), expected.trace_id);
        assert_eq!(redacted.details(), expected.details.as_ref());
    }

    #[test]
    fn try_with_trace_id_trims_whitespace_before_storing() {
        let error = Error::invalid_request_static("bad request")
            .try_with_trace_id("  trace-123  ")
            .expect("trace identifier should be valid");

        assert_eq!(error.trace_id(), Some("trace-123"));
    }

    #[test]
    fn deserialization_rejects_blank_values_after_trimming() {
        let error = serde_json::from_value::<Error>(json!({
            "code": "invalid_request",
            "message": "   ",
            "traceId": " trace-123 "
        }))
        .expect_err("blank messages should fail validation");

        assert!(
            error
                .to_string()
                .contains("error message must not be empty")
        );
    }

    #[test]
    fn deserialization_rejects_blank_trace_id_after_trimming() {
        let error = serde_json::from_value::<Error>(json!({
            "code": "invalid_request",
            "message": "bad request",
            "traceId": "   "
        }))
        .expect_err("blank trace identifiers should fail validation");

        assert!(
            error
                .to_string()
                .contains("trace identifier must not be empty")
        );
    }

    #[test]
    fn deserialization_trims_message_and_trace_id() {
        let error = serde_json::from_value::<Error>(json!({
            "code": "invalid_request",
            "message": " bad request ",
            "traceId": " trace-123 ",
            "details": {"field": "name"}
        }))
        .expect("deserialization should preserve invariants");

        assert_eq!(error.message(), "bad request");
        assert_eq!(error.trace_id(), Some("trace-123"));
        assert_eq!(error.details(), Some(&json!({"field": "name"})));
    }
}
