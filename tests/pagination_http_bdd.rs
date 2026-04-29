//! Handler-level tests for pagination HTTP error mapping.

use actix_v2a::{
    Error,
    ErrorCode,
    pagination::{Cursor, CursorError, PageParams, Paginated, PaginationLinks},
};
use actix_web::{
    App,
    HttpRequest,
    HttpResponse,
    Responder,
    test,
    web::{self, ServiceConfig},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize, Serializer};
use url::Url;

const OVERSIZED_CURSOR_LEN: usize = 8 * 1024 + 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FixtureKey {
    created_at: String,
    id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FixtureItem {
    id: String,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinksResponse {
    #[serde(rename = "self")]
    self_: String,
    next: Option<String>,
    prev: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct PaginatedResponse {
    data: Vec<FixtureItem>,
    limit: usize,
    links: LinksResponse,
}

struct FailingKey;

impl Serialize for FailingKey {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Err(serde::ser::Error::custom("fixture serialization failed"))
    }
}

#[actix_web::test]
async fn zero_limit_maps_to_invalid_request() {
    let response = dispatch("/items?limit=0").await;

    assert_error_response(
        response,
        actix_web::http::StatusCode::BAD_REQUEST,
        ErrorCode::InvalidRequest,
    )
    .await;
}

#[actix_web::test]
async fn malformed_base64_cursor_maps_to_invalid_request() {
    let response = dispatch("/items?cursor=not!valid").await;

    assert_error_response(
        response,
        actix_web::http::StatusCode::BAD_REQUEST,
        ErrorCode::InvalidRequest,
    )
    .await;
}

#[actix_web::test]
async fn json_invalid_cursor_maps_to_invalid_request() {
    let token = URL_SAFE_NO_PAD.encode(b"{}");
    let response = dispatch(&format!("/items?cursor={token}")).await;

    assert_error_response(
        response,
        actix_web::http::StatusCode::BAD_REQUEST,
        ErrorCode::InvalidRequest,
    )
    .await;
}

#[actix_web::test]
async fn oversized_cursor_maps_to_invalid_request() {
    let token = "a".repeat(OVERSIZED_CURSOR_LEN);
    let response = dispatch(&format!("/items?cursor={token}")).await;

    assert_error_response(
        response,
        actix_web::http::StatusCode::BAD_REQUEST,
        ErrorCode::InvalidRequest,
    )
    .await;
}

#[actix_web::test]
async fn cursor_serialize_maps_to_internal_error() {
    let response = dispatch("/items?forceSerializeFailure=true").await;

    assert_error_response(
        response,
        actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        ErrorCode::InternalError,
    )
    .await;
}

#[actix_web::test]
async fn valid_request_returns_paginated_fixture_items() {
    let response = dispatch("/items?limit=2").await;

    assert_eq!(response.status(), actix_web::http::StatusCode::OK);

    let body: PaginatedResponse = test::read_body_json(response).await;
    assert_eq!(body.limit, 2);
    assert_eq!(
        body.data,
        vec![
            FixtureItem {
                id: "item-1".to_owned(),
                name: "Ada".to_owned(),
            },
            FixtureItem {
                id: "item-2".to_owned(),
                name: "Grace".to_owned(),
            },
        ]
    );
    assert_eq!(body.links.self_, "http://localhost:8080/items?limit=2");
    assert!(body.links.next.is_none());
    assert!(body.links.prev.is_none());
}

async fn dispatch(uri: &str) -> actix_web::dev::ServiceResponse<actix_web::body::BoxBody> {
    let app = test::init_service(App::new().configure(configure_routes)).await;
    let request = test::TestRequest::get().uri(uri).to_request();

    test::call_service(&app, request).await
}

fn configure_routes(config: &mut ServiceConfig) {
    config.route("/items", web::get().to(paginated_items));
}

async fn paginated_items(request: HttpRequest) -> Result<impl Responder, Error> {
    let params = web::Query::<PageParams>::from_query(request.query_string())
        .map_err(|_| Error::invalid_request_static("invalid pagination parameters"))?
        .into_inner();

    if should_force_serialize_failure(request.query_string()) {
        Cursor::new(FailingKey)
            .encode()
            .map_err(|error| map_cursor_error(&error))?;
    }

    if let Some(cursor) = params.cursor() {
        Cursor::<FixtureKey>::decode(cursor).map_err(|error| map_cursor_error(&error))?;
    }

    let request_url = absolute_request_url(&request)?;
    let links = PaginationLinks::from_request(&request_url, &params, None, None);
    Ok(HttpResponse::Ok().json(Paginated::new(fixture_items(), params.limit(), links)))
}

fn map_cursor_error(error: &CursorError) -> Error {
    match error {
        CursorError::Serialize { .. } => Error::internal_static("cursor serialization failed"),
        CursorError::InvalidBase64 { .. }
        | CursorError::Deserialize { .. }
        | CursorError::TokenTooLong { .. } => {
            Error::invalid_request_static("invalid pagination cursor")
        }
    }
}

fn should_force_serialize_failure(query_string: &str) -> bool {
    url::form_urlencoded::parse(query_string.as_bytes())
        .any(|(key, value)| key == "forceSerializeFailure" && value == "true")
}

fn absolute_request_url(request: &HttpRequest) -> Result<Url, Error> {
    let connection = request.connection_info();
    let path_and_query = request.uri().path_and_query().map_or_else(
        || request.uri().path(),
        actix_web::http::uri::PathAndQuery::as_str,
    );
    let absolute_url = format!(
        "{}://{}{}",
        connection.scheme(),
        connection.host(),
        path_and_query
    );

    Url::parse(&absolute_url).map_err(|_| Error::internal_static("invalid request URI"))
}

fn fixture_items() -> Vec<FixtureItem> {
    vec![
        FixtureItem {
            id: "item-1".to_owned(),
            name: "Ada".to_owned(),
        },
        FixtureItem {
            id: "item-2".to_owned(),
            name: "Grace".to_owned(),
        },
    ]
}

async fn assert_error_response(
    response: actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
    expected_status: actix_web::http::StatusCode,
    expected_code: ErrorCode,
) {
    assert_eq!(response.status(), expected_status);

    let body: Error = test::read_body_json(response).await;
    assert_eq!(body.code(), expected_code);
}
