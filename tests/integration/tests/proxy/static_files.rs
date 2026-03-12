use integration::conf::{ConfigBuilder, minimal_static_file_runtime_config};
use integration::constants::{HTTP_RESPONSE_BODY, ROUTE_PATH_API};
use integration::harness::TestServer;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;

const INDEX_HTML: &str = "index.html";
const IMAGES_DIR: &str = "images";
const IMAGES_1MB_PNG: &str = "1mb.png";

/// Serves index.html from the configured static directory
#[test]
fn serves_index_html_from_static_dir() {
    let mut cfg = minimal_static_file_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv.get(INDEX_HTML).send().unwrap();

    let status = res.status();
    let body = res.text().unwrap();

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("Snakeway"),
        "unexpected response body: {body}"
    );
}

/// Static routes should not require an upstream to be available
#[test]
fn static_route_does_not_require_upstream() {
    let mut cfg = minimal_static_file_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv.get(INDEX_HTML).send().unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

/// Proxy routes should still work when static file serving is enabled
#[test]
fn proxy_route_still_works_when_static_is_enabled() {
    let mut cfg = ConfigBuilder::default()
        .with_static_file_and_service_ingress()
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv.get(ROUTE_PATH_API).send().unwrap();

    let status = res.status();
    let body = res.text().unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, HTTP_RESPONSE_BODY);
}

#[test]
fn static_path_traversal_is_rejected() {
    let mut cfg = minimal_static_file_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let inaccessible_path = "/static/../Cargo.toml";

    let res = srv.get(inaccessible_path).send().unwrap();

    assert!(
        res.status().is_client_error(),
        "expected client error, got {}",
        res.status()
    );
}

#[test]
fn static_response_includes_cache_headers() {
    let mut cfg = minimal_static_file_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv.get(INDEX_HTML).send().unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let headers = res.headers();

    assert!(
        headers.contains_key(reqwest::header::CACHE_CONTROL),
        "Cache-Control header missing"
    );
    assert!(
        headers.contains_key(reqwest::header::ETAG),
        "ETag header missing"
    );
    assert!(
        headers.contains_key(reqwest::header::LAST_MODIFIED),
        "Last-Modified header missing"
    );
}

#[test]
fn if_none_match_returns_304() {
    let mut cfg = minimal_static_file_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let initial = srv.get(INDEX_HTML).send().unwrap();

    let etag = initial
        .headers()
        .get(reqwest::header::ETAG)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let client = reqwest::blocking::Client::new();
    let res = client
        .get(format!("{}/index.html", srv.base_url()))
        .header(reqwest::header::IF_NONE_MATCH, etag)
        .send()
        .unwrap();

    assert_eq!(res.status(), reqwest::StatusCode::NOT_MODIFIED);
    assert!(res.text().unwrap().is_empty());
}

#[test]
fn directory_listing_renders_when_enabled() {
    let mut cfg = ConfigBuilder::default()
        .with_static_file_ingress(true)
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let target_path = format!("/{}/", IMAGES_DIR);

    let res = srv.get(&target_path).send().unwrap();

    let status = res.status();
    let body = res.text().unwrap();
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Index of /"));
}

#[test]
fn directory_listing_includes_expected_file() {
    let mut cfg = ConfigBuilder::default()
        .with_static_file_ingress(true)
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let body = srv.get(IMAGES_DIR).send().unwrap().text().unwrap();

    assert!(body.contains(IMAGES_1MB_PNG));
}

#[test]
fn supports_range_requests() {
    let mut cfg = ConfigBuilder::default()
        .with_static_file_ingress(true)
        .build();
    let _ = TestServer::start_http_upstream_with_config(&mut cfg);
    let target_path = format!("{}/{}", IMAGES_DIR, IMAGES_1MB_PNG);

    let client = reqwest::blocking::Client::new();
    let res = client
        .get(target_path)
        .header(reqwest::header::RANGE, "bytes=0-99")
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::PARTIAL_CONTENT);

    let content_range = res
        .headers()
        .get(reqwest::header::CONTENT_RANGE)
        .unwrap()
        .to_str()
        .unwrap();

    assert!(content_range.starts_with("bytes 0-99/"));

    let body = res.bytes().unwrap();
    assert_eq!(body.len(), 100);
}

#[test]
fn head_request_returns_headers_without_body() {
    let spec = ConfigBuilder::default().with_static_file_ingress(true);
    let mut cfg = spec.build();
    let _ = TestServer::start_http_upstream_with_config(&mut cfg);
    let target_path = format!("{}/{}", IMAGES_DIR, IMAGES_1MB_PNG);
    let client = reqwest::blocking::Client::new();

    let res = client.head(target_path).send().unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.headers().contains_key(reqwest::header::CONTENT_LENGTH));
    assert!(res.headers().contains_key(reqwest::header::ACCEPT_RANGES));

    let body = res.bytes().unwrap();
    assert!(body.is_empty());
}
