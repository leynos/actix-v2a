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

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode { status_for(self.code()) }

    fn error_response(&self) -> HttpResponse {
        let payload = self.redacted();
        let mut builder = HttpResponse::build(self.status_code());
        if let Some(trace_id) = payload.trace_id() {
            builder.insert_header((TRACE_ID_HEADER, trace_id.to_owned()));
        }

        builder.json(payload)
    }
}

impl From<actix_web::Error> for Error {
    fn from(error: actix_web::Error) -> Self {
        error!(error = %error, "actix error promoted to shared API error");
        Self::internal_static("Internal server error")
    }
}

#[cfg(test)]
mod tests {
    //! Regression coverage for Actix error responders.

    use actix_web::{ResponseError, body::to_bytes, http::StatusCode};
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
                Error::from_static(ErrorCode::Unauthorized, "no auth"),
                StatusCode::UNAUTHORIZED,
            ),
            (
                Error::from_static(ErrorCode::Forbidden, "denied"),
                StatusCode::FORBIDDEN,
            ),
            (
                Error::from_static(ErrorCode::NotFound, "missing"),
                StatusCode::NOT_FOUND,
            ),
            (Error::conflict_static("duplicate"), StatusCode::CONFLICT),
            (
                Error::from_static(ErrorCode::ServiceUnavailable, "db unavailable"),
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

    #[test]
    fn actix_errors_are_promoted_to_redacted_internal_errors() {
        let actix_error = actix_web::error::ErrorBadRequest("boom");
        let error: Error = actix_error.into();

        assert_eq!(error.code(), ErrorCode::InternalError);
        assert_eq!(error.message(), "Internal server error");
        assert_eq!(error.trace_id(), None);
    }
}
