//! Crate-root Actix adapter wire-contract tests.

use actix_v2a::{
    EVENT_STREAM_CACHE_CONTROL,
    LAST_EVENT_ID_HEADER,
    apply_actix_event_stream_cache_control,
    extract_actix_replay_cursor,
};
use actix_web::http::header::{CACHE_CONTROL, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};

#[test]
fn documented_crate_root_actix_adapter_imports_match_wire_contract() {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_bytes(LAST_EVENT_ID_HEADER.as_bytes())
            .expect("LAST_EVENT_ID_HEADER should be a valid Actix header name"),
        HeaderValue::from_static("evt-crate-root"),
    );

    apply_actix_event_stream_cache_control(&mut headers);
    let replay_cursor = extract_actix_replay_cursor(&headers)
        .expect("documented replay cursor import should parse");

    assert_eq!(
        headers
            .get(CACHE_CONTROL)
            .expect("cache-control header should be present"),
        EVENT_STREAM_CACHE_CONTROL
    );
    assert_eq!(
        replay_cursor
            .expect("replay cursor should be present")
            .as_ref(),
        "evt-crate-root"
    );
}

#[test]
fn documented_crate_root_actix_adapter_import_rejects_non_utf8_cursor() {
    let mut headers = HeaderMap::new();
    let non_utf8_bytes = &[0xff, 0xfe, 0xfd];
    #[expect(
        unsafe_code,
        reason = "Test needs to construct invalid UTF-8 header value"
    )]
    // SAFETY: `HeaderValue::from_maybe_shared_unchecked(non_utf8_bytes)` is
    // used here because this integration test must exercise the public
    // adapter's non-UTF-8 header path, which safe constructors reject. The
    // bytes are static, test-only raw header bytes with no null bytes and no
    // invalid internal representation for this controlled `HeaderValue`.
    let header_value = unsafe { HeaderValue::from_maybe_shared_unchecked(non_utf8_bytes) };
    headers.insert(
        HeaderName::from_bytes(LAST_EVENT_ID_HEADER.as_bytes())
            .expect("LAST_EVENT_ID_HEADER should be a valid Actix header name"),
        header_value,
    );

    assert!(
        extract_actix_replay_cursor(&headers).is_err(),
        "documented crate-root import should reject non-UTF-8 Last-Event-ID values"
    );
}

#[test]
fn documented_crate_root_actix_adapter_import_returns_none_without_cursor() {
    let headers = HeaderMap::new();

    let replay_cursor = extract_actix_replay_cursor(&headers)
        .expect("documented crate-root import should accept missing Last-Event-ID");

    assert_eq!(replay_cursor, None);
}

#[test]
fn documented_crate_root_actix_adapter_import_replaces_cache_policy() {
    let mut headers = HeaderMap::new();
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=60"),
    );

    apply_actix_event_stream_cache_control(&mut headers);

    assert_eq!(
        headers
            .get(CACHE_CONTROL)
            .expect("cache-control header should be present"),
        EVENT_STREAM_CACHE_CONTROL
    );
}

#[test]
fn documented_crate_root_actix_adapter_import_preserves_unrelated_headers() {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        HeaderName::from_bytes(LAST_EVENT_ID_HEADER.as_bytes())
            .expect("LAST_EVENT_ID_HEADER should be a valid Actix header name"),
        HeaderValue::from_static("evt-crate-root"),
    );

    apply_actix_event_stream_cache_control(&mut headers);

    assert_eq!(
        headers
            .get(CONTENT_TYPE)
            .expect("content-type header should be present"),
        "application/json"
    );
}
