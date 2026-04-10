use integration::conf::{ConfigBuilder, minimal_static_file_runtime_config};
use integration::constants::{HTTP_RESPONSE_BODY, ROUTE_PATH_API, TEST_HOST};
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

    let res = srv.get(INDEX_HTML).send().expect("failed to get HTML page");

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
    let target_path = format!("/{}/", IMAGES_DIR);

    let body = srv.get(&target_path).send().unwrap().text().unwrap();

    assert!(body.contains(IMAGES_1MB_PNG));
}

#[test]
fn supports_range_requests() {
    let mut cfg = ConfigBuilder::default()
        .with_static_file_ingress(true)
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let target_path = format!("{}/{}/{}", srv.base_url(), IMAGES_DIR, IMAGES_1MB_PNG);
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
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let target_path = format!("{}/{}/{}", srv.base_url(), IMAGES_DIR, IMAGES_1MB_PNG);
    let client = reqwest::blocking::Client::new();

    let res = client.head(target_path).send().unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.headers().contains_key(reqwest::header::CONTENT_LENGTH));
    assert!(res.headers().contains_key(reqwest::header::ACCEPT_RANGES));

    let body = res.bytes().unwrap();
    assert!(body.is_empty());
}

/// When gzip compression is enabled and the client sends `Accept-Encoding: gzip`,
/// the response for a qualifying file must include `Content-Encoding: gzip` and
/// the body must be valid gzip data.
#[test]
fn static_file_gzip_compression_negotiation() {
    // Arrange: minimal_static_file_runtime_config already has enable_gzip=true
    // and min_gzip_size=1024. index.html is 921 bytes, which is below 1024.
    // So we build a custom config with min_gzip_size=64 to ensure it qualifies.
    let mut cfg = ConfigBuilder::default()
        .with_static_file_ingress(false)
        .build();

    // Patch compression settings so index.html (921 bytes) qualifies for gzip.
    // small_file_threshold must be >= file size (files below this are compressed in-memory).
    // min_gzip_size must be <= file size.
    for route in &mut cfg.routes {
        if let snakeway_core::testing_api::conf::types::RouteConfig::Static(sr) = route {
            sr.static_config.small_file_threshold = 1_048_576;
            sr.static_config.min_gzip_size = 64;
        }
    }

    let srv = TestServer::start_with_config(&mut cfg, |_port| {
        // Static files don't need an upstream listener.
    });

    // Use a client that does NOT auto-decompress gzip.
    let client = reqwest::blocking::Client::builder()
        .no_gzip()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();

    // Act
    let res = client
        .get(format!("{}/{INDEX_HTML}", srv.base_url()))
        .header("Accept-Encoding", "gzip")
        .header("Host", TEST_HOST)
        .send()
        .unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);

    let content_encoding = res
        .headers()
        .get(reqwest::header::CONTENT_ENCODING)
        .expect("Content-Encoding header must be present")
        .to_str()
        .unwrap();
    assert_eq!(content_encoding, "gzip", "Content-Encoding should be gzip");

    let compressed_body = res.bytes().unwrap();
    assert!(
        compressed_body.len() >= 2 && compressed_body[0] == 0x1f && compressed_body[1] == 0x8b,
        "response body must start with gzip magic bytes"
    );
}
