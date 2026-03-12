use integration::conf::{
    ConfigBuilder, minimal_http_runtime_config, minimal_http_runtime_config_with_request_filter,
};
use integration::harness::TestServer;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;

#[test]
fn request_filter_disabled_allows_request() {
    let expected = StatusCode::OK;
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv.get("/api").send().unwrap();

    assert_eq!(res.status(), expected);
}

#[test]
fn request_filter_allows_get_method() {
    let mut cfg = minimal_http_runtime_config_with_request_filter();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

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
    let mut cfg = minimal_http_runtime_config_with_request_filter();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv.put("/api").send().unwrap();

    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[test]
fn request_filter_deny_methods_take_precedence() {
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_request_filter_device_that_denies_get_method()
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv.get("/api").send().unwrap();

    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[test]
fn request_filter_denies_forbidden_header() {
    let mut cfg = minimal_http_runtime_config_with_request_filter();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv
        .get("/api")
        .header("x-forwarded-host", "evil.example")
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[test]
fn request_filter_requires_a_header_that_is_not_provided() {
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_request_filter_device_that_requires_header()
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv.get("/api").send().unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn request_filter_allows_only_whitelisted_headers() {
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_request_filter_device_that_allows_specific_headers()
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv
        .get("/api")
        .header("x-custom-allowed", "ok")
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

#[test]
fn request_filter_blocks_non_whitelisted_headers() {
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_request_filter_device_that_allows_specific_headers()
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv
        .get("/api")
        .header("x-not-allowed", "nope")
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[test]
fn request_filter_enforces_header_size_limit() {
    let mut cfg = minimal_http_runtime_config_with_request_filter();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let big_value = "a".repeat(2048);

    let res = srv.get("/api").header("x-big", big_value).send().unwrap();

    assert_eq!(res.status(), StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE);
}

#[test]
fn request_filter_enforces_body_size_limit() {
    let mut cfg = minimal_http_runtime_config_with_request_filter();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv.post("/api").body(vec![0u8; 20_000]).send().unwrap();

    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[test]
fn request_filter_enforces_suspicious_body_size_limit() {
    let mut cfg = minimal_http_runtime_config_with_request_filter();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv.delete("/api").body(vec![0u8; 20_000]).send().unwrap();

    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[test]
fn request_filter_uses_custom_deny_status() {
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_request_filter_device_that_overrides_deny_status()
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv.delete("/api").send().unwrap();

    assert_eq!(res.status(), StatusCode::NOT_ACCEPTABLE);
}
