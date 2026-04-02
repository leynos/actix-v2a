//! Actix responder support for the shared API error envelope.

use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use tracing::error;

use crate::{Error, ErrorCode, TRACE_ID_HEADER};

/// Convenient result alias for Actix handlers.
pub type ApiResult<T> = Result<T, Error>;

const fn status_for(code: ErrorCode) -> StatusCode {
    match code {
        ErrorCode::InvalidRequest => StatusCode::BAD_REQUEST,
        ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
        ErrorCode::Forbidden => StatusCode::FORBIDDEN,
        ErrorCode::NotFound => StatusCode::NOT_FOUND,
        ErrorCode::Conflict => StatusCode::CONFLICT,
        ErrorCode::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        ErrorCode::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn code_for_status(status: StatusCode) -> Option<ErrorCode> {
    match status {
        StatusCode::BAD_REQUEST => Some(ErrorCode::InvalidRequest),
        StatusCode::UNAUTHORIZED => Some(ErrorCode::Unauthorized),
        StatusCode::FORBIDDEN => Some(ErrorCode::Forbidden),
        StatusCode::NOT_FOUND => Some(ErrorCode::NotFound),
        StatusCode::CONFLICT => Some(ErrorCode::Conflict),
        StatusCode::SERVICE_UNAVAILABLE => Some(ErrorCode::ServiceUnavailable),
        StatusCode::INTERNAL_SERVER_ERROR => Some(ErrorCode::InternalError),
        _ => None,
    }
}

fn redact_server_error(status: StatusCode) -> Error {
    match code_for_status(status) {
        Some(ErrorCode::ServiceUnavailable) => {
            Error::from_static(ErrorCode::ServiceUnavailable, "Service unavailable")
                .unwrap_or_else(|_| Error::fallback_for(ErrorCode::ServiceUnavailable))
        }
        Some(ErrorCode::InternalError) => Error::internal_static("Internal server error"),
        _ => Error::internal_static("Internal server error").with_http_status(status.as_u16()),
    }
}

fn preserve_client_error(error: &actix_web::Error, status: StatusCode) -> Error {
    let mapped_code = code_for_status(status);
    let code = mapped_code.unwrap_or(ErrorCode::InvalidRequest);
    let message = error.to_string();

    let payload = Error::try_new(code, message).unwrap_or_else(|_| {
        Error::from_static(code, status.canonical_reason().unwrap_or("Request failed"))
            .unwrap_or_else(|_| Error::fallback_for(code))
    });

    if mapped_code.is_none() {
        return payload.with_http_status(status.as_u16());
    }

    payload
}

fn adapt_server_error(error: &actix_web::Error, status: StatusCode) -> Error {
    error!(error = %error, %status, "actix error promoted to shared API error");
    redact_server_error(status)
}

fn adapt_client_error(error: &actix_web::Error, status: StatusCode) -> Error {
    error!(%status, "actix error promoted to shared API error");
    preserve_client_error(error, status)
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        self.http_status()
            .and_then(|status| StatusCode::from_u16(status).ok())
            .unwrap_or_else(|| status_for(self.code()))
    }

    fn error_response(&self) -> HttpResponse {
        let payload = self.redacted();
        let mut builder = HttpResponse::build(self.status_code());
        if let Some(trace_id) = payload.trace_id() {
            builder.insert_header((TRACE_ID_HEADER, trace_id.to_owned()));
        }

        builder.json(payload)
    }
}

fn adapt_actix_error(error: &actix_web::Error, status: StatusCode) -> Error {
    if status.is_server_error() {
        return adapt_server_error(error, status);
    }

    adapt_client_error(error, status)
}

impl From<actix_web::Error> for Error {
    fn from(error: actix_web::Error) -> Self {
        let status = error.as_response_error().status_code();
        adapt_actix_error(&error, status)
    }
}

#[cfg(test)]
mod tests {
    //! Regression coverage for Actix error responders.

    use actix_web::{ResponseError, body::to_bytes, http::StatusCode};
    use rstest::rstest;
    use serde_json::json;

    use crate::{Error, ErrorCode, TRACE_ID_HEADER};

    async fn decode_response(
        error: Error,
        expected_status: StatusCode,
        expected_trace_id: Option<&str>,
    ) -> Error {
        let response = error.error_response();
        assert_eq!(response.status(), expected_status);

        let trace_header = response.headers().get(TRACE_ID_HEADER);
        match expected_trace_id {
            Some(expected) => {
                let trace_id = trace_header
                    .expect("trace header should be present")
                    .to_str()
                    .expect("trace header should be UTF-8");
                assert_eq!(trace_id, expected);
            }
            None => assert!(trace_header.is_none()),
        }

        let body = to_bytes(response.into_body())
            .await
            .expect("response body should be readable");

        serde_json::from_slice(&body).expect("error payload should deserialize")
    }

    #[test]
    fn status_code_matches_error_code() {
        let cases = [
            (
                Error::invalid_request_static("bad"),
                StatusCode::BAD_REQUEST,
            ),
            (
                Error::from_static(ErrorCode::Unauthorized, "no auth")
                    .expect("static message should be valid"),
                StatusCode::UNAUTHORIZED,
            ),
            (
                Error::from_static(ErrorCode::Forbidden, "denied")
                    .expect("static message should be valid"),
                StatusCode::FORBIDDEN,
            ),
            (
                Error::from_static(ErrorCode::NotFound, "missing")
                    .expect("static message should be valid"),
                StatusCode::NOT_FOUND,
            ),
            (Error::conflict_static("duplicate"), StatusCode::CONFLICT),
            (
                Error::from_static(ErrorCode::ServiceUnavailable, "db unavailable")
                    .expect("static message should be valid"),
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            (
                Error::internal_static("boom"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];

        for (error, status) in cases {
            assert_eq!(ResponseError::status_code(&error), status);
        }
    }

    #[actix_web::test]
    async fn internal_errors_are_redacted_in_http_responses() {
        let error = Error::internal_static("boom")
            .try_with_trace_id("trace-123")
            .expect("trace identifier should be valid")
            .with_details(json!({"secret": true}));

        let payload =
            decode_response(error, StatusCode::INTERNAL_SERVER_ERROR, Some("trace-123")).await;

        assert_eq!(payload.code(), ErrorCode::InternalError);
        assert_eq!(payload.message(), "Internal server error");
        assert_eq!(payload.trace_id(), Some("trace-123"));
        assert_eq!(payload.details(), None);
    }

    #[actix_web::test]
    async fn non_internal_errors_keep_message_and_details() {
        let error = Error::invalid_request_static("bad")
            .try_with_trace_id("trace-456")
            .expect("trace identifier should be valid")
            .with_details(json!({"field": "name"}));

        let payload = decode_response(error, StatusCode::BAD_REQUEST, Some("trace-456")).await;

        assert_eq!(payload.code(), ErrorCode::InvalidRequest);
        assert_eq!(payload.message(), "bad");
        assert_eq!(payload.trace_id(), Some("trace-456"));
        assert_eq!(payload.details(), Some(&json!({"field": "name"})));
    }

    fn bad_request_error() -> actix_web::Error { actix_web::error::ErrorBadRequest("boom") }

    fn not_found_error() -> actix_web::Error { actix_web::error::ErrorNotFound("missing route") }

    fn unmapped_client_error() -> actix_web::Error {
        actix_web::error::InternalError::new("teapot", StatusCode::IM_A_TEAPOT).into()
    }

    fn service_unavailable_error() -> actix_web::Error {
        actix_web::error::ErrorServiceUnavailable("db unavailable")
    }

    fn internal_server_error() -> actix_web::Error {
        actix_web::error::ErrorInternalServerError("boom")
    }

    fn unmapped_server_error() -> actix_web::Error {
        actix_web::error::InternalError::new("gateway timeout", StatusCode::GATEWAY_TIMEOUT).into()
    }

    #[rstest]
    #[case(
        bad_request_error,
        ErrorCode::InvalidRequest,
        Some("boom"),
        Some(StatusCode::BAD_REQUEST)
    )]
    #[case(
        not_found_error,
        ErrorCode::NotFound,
        Some("missing route"),
        Some(StatusCode::NOT_FOUND)
    )]
    #[case(
        unmapped_client_error,
        ErrorCode::InvalidRequest,
        Some("teapot"),
        Some(StatusCode::IM_A_TEAPOT)
    )]
    #[case(
        service_unavailable_error,
        ErrorCode::ServiceUnavailable,
        Some("Service unavailable"),
        Some(StatusCode::SERVICE_UNAVAILABLE)
    )]
    #[case(
        internal_server_error,
        ErrorCode::InternalError,
        Some("Internal server error"),
        Some(StatusCode::INTERNAL_SERVER_ERROR)
    )]
    #[case(
        unmapped_server_error,
        ErrorCode::InternalError,
        Some("Internal server error"),
        Some(StatusCode::GATEWAY_TIMEOUT)
    )]
    fn actix_error_conversion(
        #[case] make_error: fn() -> actix_web::Error,
        #[case] expected_code: ErrorCode,
        #[case] expected_public_message: Option<&str>,
        #[case] expected_status: Option<StatusCode>,
    ) {
        let error: Error = make_error().into();

        assert_eq!(error.code(), expected_code);
        if let Some(expected_message) = expected_public_message {
            assert_eq!(error.message(), expected_message);
        }
        if let Some(status) = expected_status {
            assert_eq!(ResponseError::status_code(&error), status);
        }
        assert_eq!(error.trace_id(), None);
    }
}
