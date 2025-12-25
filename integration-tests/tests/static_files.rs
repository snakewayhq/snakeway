use integration_tests::harness::TestServer;
use reqwest::StatusCode;

/// Serves index.html from the configured static directory
#[test]
fn serves_index_html_from_static_dir() {
    let srv = TestServer::start("static");

    let res = srv.get("/index.html").send().unwrap();

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
    let srv = TestServer::start("static");

    let res = srv.get("/index.html").send().unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

/// Proxy routes should still work when static file serving is enabled
#[test]
fn proxy_route_still_works_when_static_is_enabled() {
    let srv = TestServer::start("static");

    let res = srv.get("/api").send().unwrap();

    let status = res.status();
    let body = res.text().unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "hello world");
}

#[test]
fn static_path_traversal_is_rejected() {
    let srv = TestServer::start("static");

    let res = srv.get("/static/../Cargo.toml").send().unwrap();

    assert!(
        res.status().is_client_error(),
        "expected client error, got {}",
        res.status()
    );
}

#[test]
fn static_response_includes_cache_headers() {
    let srv = TestServer::start("static");

    let res = srv.get("/index.html").send().unwrap();

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
    let srv = TestServer::start("static");

    let initial = srv.get("/index.html").send().unwrap();

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
    let srv = TestServer::start("static_nondefault");

    let res = srv.get("/images/").send().unwrap();

    let status = res.status();
    let body = res.text().unwrap();

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Index of /"));
}

#[test]
fn directory_listing_includes_expected_file() {
    let srv = TestServer::start("static_nondefault");

    let body = srv.get("/images/").send().unwrap().text().unwrap();

    assert!(body.contains("41kb.png"));
}

#[test]
fn supports_range_requests() {
    let srv = TestServer::start("static_nondefault");

    let client = reqwest::blocking::Client::new();
    let res = client
        .get(format!("{}/images/41kb.png", srv.base_url()))
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
    let srv = TestServer::start("static_nondefault");

    let client = reqwest::blocking::Client::new();
    let res = client
        .head(format!("{}/images/41kb.png", srv.base_url()))
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.headers().contains_key(reqwest::header::CONTENT_LENGTH));
    assert!(res.headers().contains_key(reqwest::header::ACCEPT_RANGES));

    let body = res.bytes().unwrap();
    assert!(body.is_empty());
}
