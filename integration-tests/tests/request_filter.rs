use integration_tests::harness::TestServer;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;

#[test]
fn request_filter_disabled_allows_request() {
    let srv = TestServer::start_with_http_upstream("request_filter_disabled");

    let res = srv.get("/api").send().unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

#[test]
fn request_filter_allows_get_method() {
    let srv = TestServer::start_with_http_upstream("request_filter");

    let res = srv
        .get("/api")
        .header(
            "user-agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
        )
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

#[test]
fn request_filter_denies_disallowed_method() {
    let srv = TestServer::start_with_http_upstream("request_filter");

    let res = srv.put("/api").send().unwrap();

    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[test]
fn request_filter_deny_methods_take_precedence() {
    let srv = TestServer::start_with_http_upstream("request_filter_deny_get");

    let res = srv.get("/api").send().unwrap();

    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[test]
fn request_filter_denies_forbidden_header() {
    let srv = TestServer::start_with_http_upstream("request_filter");

    let res = srv
        .get("/api")
        .header("x-forwarded-host", "evil.example")
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[test]
fn request_filter_requires_a_header() {
    let srv = TestServer::start_with_http_upstream("request_filter");

    let res = srv
        .get("/api")
        .header("x-required", "need-this")
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn request_filter_allows_only_whitelisted_headers() {
    let srv = TestServer::start_with_http_upstream("request_filter_allow_headers");

    let res = srv
        .get("/api")
        .header("x-custom-allowed", "ok")
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

#[test]
fn request_filter_blocks_non_whitelisted_headers() {
    let srv = TestServer::start_with_http_upstream("request_filter_allow_headers");

    let res = srv
        .get("/api")
        .header("x-not-allowed", "nope")
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[test]
fn request_filter_enforces_header_size_limit() {
    let srv = TestServer::start_with_http_upstream("request_filter");
    let big_value = "a".repeat(2048);

    let res = srv.get("/api").header("x-big", big_value).send().unwrap();

    assert_eq!(res.status(), StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE);
}

#[test]
fn request_filter_enforces_body_size_limit() {
    let srv = TestServer::start_with_http_upstream("request_filter");

    let res = srv.post("/api").body(vec![0u8; 20_000]).send().unwrap();

    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[test]
fn request_filter_enforces_suspicious_body_size_limit() {
    let srv = TestServer::start_with_http_upstream("request_filter");

    let res = srv.delete("/api").body(vec![0u8; 20_000]).send().unwrap();

    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[test]
fn request_filter_uses_custom_deny_status() {
    let srv = TestServer::start_with_http_upstream("request_filter_custom_status");

    let res = srv.delete("/api").send().unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
