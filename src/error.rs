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
    #[serde(skip)]
    http_status: Option<u16>,
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
    const fn fallback_message_for(code: ErrorCode) -> &'static str {
        match code {
            ErrorCode::InvalidRequest => "Invalid request",
            ErrorCode::Unauthorized => "Unauthorized",
            ErrorCode::Forbidden => "Forbidden",
            ErrorCode::NotFound => "Not found",
            ErrorCode::Conflict => "Conflict",
            ErrorCode::ServiceUnavailable => "Service unavailable",
            ErrorCode::InternalError => "Internal server error",
        }
    }

    pub(crate) fn fallback_for(code: ErrorCode) -> Self {
        Self {
            code,
            message: Self::fallback_message_for(code).to_owned(),
            trace_id: None,
            details: None,
            http_status: None,
        }
    }

    fn normalize_non_empty_text(
        input_text: impl Into<String>,
        empty_error: ErrorValidationError,
    ) -> Result<String, ErrorValidationError> {
        let owned_text = input_text.into();
        let trimmed_text = owned_text.trim();
        if trimmed_text.is_empty() {
            return Err(empty_error);
        }

        Ok(trimmed_text.to_owned())
    }

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
        let normalized_message =
            Self::normalize_non_empty_text(message, ErrorValidationError::EmptyMessage)?;

        Ok(Self {
            code,
            message: normalized_message,
            trace_id: None,
            details: None,
            http_status: None,
        })
    }

    fn validated_static_or_fallback(code: ErrorCode, message: &'static str) -> Self {
        Self::from_static(code, message).unwrap_or_else(|_| Self::fallback_for(code))
    }

    /// Construct a library-owned error payload from a trusted static message.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorValidationError::EmptyMessage`] when `message` is blank
    /// once trimmed.
    pub fn from_static(
        code: ErrorCode,
        message: &'static str,
    ) -> Result<Self, ErrorValidationError> {
        Self::try_new(code, message)
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

    #[must_use]
    pub(crate) const fn http_status(&self) -> Option<u16> { self.http_status }

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
        self.trace_id = Some(Self::normalize_non_empty_text(
            trace_id,
            ErrorValidationError::EmptyTraceId,
        )?);
        Ok(self)
    }

    /// Attach structured details to the payload.
    #[must_use]
    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    #[must_use]
    pub(crate) const fn with_http_status(mut self, http_status: u16) -> Self {
        self.http_status = Some(http_status);
        self
    }

    /// Construct an invalid-request error from a trusted static message.
    #[must_use]
    pub fn invalid_request_static(message: &'static str) -> Self {
        Self::validated_static_or_fallback(ErrorCode::InvalidRequest, message)
    }

    /// Construct a conflict error from a trusted static message.
    #[must_use]
    pub fn conflict_static(message: &'static str) -> Self {
        Self::validated_static_or_fallback(ErrorCode::Conflict, message)
    }

    /// Construct an internal error from a trusted static message.
    #[must_use]
    pub fn internal_static(message: &'static str) -> Self {
        Self::validated_static_or_fallback(ErrorCode::InternalError, message)
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
    fn try_new_trims_whitespace_before_storing() {
        let error =
            Error::try_new(ErrorCode::InvalidRequest, " bad request ").expect("message is valid");
        assert_eq!(error.message(), "bad request");
    }

    #[test]
    fn from_static_trims_whitespace_before_storing() {
        let error = Error::from_static(ErrorCode::InvalidRequest, " bad request ")
            .expect("message is valid");

        assert_eq!(error.message(), "bad request");
    }

    #[test]
    fn from_static_rejects_blank_messages() {
        let result = Error::from_static(ErrorCode::InvalidRequest, "   ");
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

    #[rstest]
    #[case(
        json!({
            "code": "invalid_request",
            "message": "   ",
            "traceId": " trace-123 "
        }),
        "error message must not be empty"
    )]
    #[case(
        json!({
            "code": "invalid_request",
            "message": "bad request",
            "traceId": "   "
        }),
        "trace identifier must not be empty"
    )]
    fn deserialization_rejects_blank_values_after_trimming(
        #[case] payload: Value,
        #[case] expected_error_substring: &str,
    ) {
        let error = serde_json::from_value::<Error>(payload)
            .expect_err("blank values should fail validation");
        assert!(error.to_string().contains(expected_error_substring));
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
