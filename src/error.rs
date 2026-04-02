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
        if trace_id_text.trim().is_empty() {
            return Err(ErrorValidationError::EmptyTraceId);
        }

        self.trace_id = Some(trace_id_text);
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

    use serde_json::json;

    use super::{Error, ErrorCode, ErrorValidationError};

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

    #[test]
    fn redacted_internal_error_hides_message_and_details() {
        let error = Error::internal_static("boom")
            .try_with_trace_id("trace-123")
            .expect("trace identifier should be valid")
            .with_details(json!({"secret": true}));

        let redacted = error.redacted();

        assert_eq!(redacted.code(), ErrorCode::InternalError);
        assert_eq!(redacted.message(), "Internal server error");
        assert_eq!(redacted.trace_id(), Some("trace-123"));
        assert_eq!(redacted.details(), None);
    }

    #[test]
    fn non_internal_errors_keep_message_and_details() {
        let error =
            Error::invalid_request_static("bad request").with_details(json!({"field": "name"}));

        let redacted = error.redacted();

        assert_eq!(redacted.message(), "bad request");
        assert_eq!(redacted.details(), Some(&json!({"field": "name"})));
    }
}
