mod common;

use std::sync::Once;

static SERVER: Once = Once::new();
static CONFIG: &str = "static.toml";

#[test]
fn serves_index_html_from_static_dir() {
    // Arrange
    common::start_server(&SERVER, CONFIG);

    // Act
    let res = reqwest::blocking::get("http://127.0.0.1:4041/").expect("static request failed");

    let status = res.status();
    let body = res.text().expect("failed to read response body");

    // Assert
    assert_eq!(status, 200);
    assert!(
        body.contains("Snakeway"),
        "unexpected response body: {body}"
    );
}

#[test]
fn static_route_does_not_require_upstream() {
    // Arrange
    // NOTE: intentionally NOT spawning upstream
    common::start_server(&SERVER, CONFIG);

    // Act
    let res = reqwest::blocking::get("http://127.0.0.1:4041/").expect("static request failed");

    // Assert
    assert_eq!(res.status(), 200);
}

#[test]
fn proxy_route_still_works_when_static_is_enabled() {
    // Arrange
    common::start_upstream();
    common::start_server(&SERVER, CONFIG);

    // Act
    let res = reqwest::blocking::get("http://127.0.0.1:4041/api").expect("proxy request failed");

    let status = res.status();
    let body = res.text().expect("failed to read response body");

    // Assert
    assert_eq!(status, 200);
    assert_eq!(body, "hello world");
}

#[test]
fn static_path_traversal_is_rejected() {
    // Arrange
    common::start_server(&SERVER, CONFIG);

    // Act
    let res = reqwest::blocking::get("http://127.0.0.1:4041/static/../Cargo.toml")
        .expect("request failed");

    // Assert
    assert!(
        res.status().is_client_error(),
        "expected client error, got {}",
        res.status()
    );
}

#[test]
fn static_response_includes_cache_headers() {
    // Arrange
    common::start_server(&SERVER, CONFIG);

    // Act
    let res = reqwest::blocking::get("http://127.0.0.1:4041/").expect("static request failed");

    // Assert
    assert_eq!(res.status(), 200);

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
    // Arrange
    common::start_server(&SERVER, CONFIG);

    let initial = reqwest::blocking::get("http://127.0.0.1:4041/").expect("initial request failed");

    let etag = initial
        .headers()
        .get(reqwest::header::ETAG)
        .expect("ETag missing")
        .to_str()
        .unwrap()
        .to_string();

    // Act
    let client = reqwest::blocking::Client::new();
    let res = client
        .get("http://127.0.0.1:4041/")
        .header(reqwest::header::IF_NONE_MATCH, etag)
        .send()
        .expect("conditional request failed");

    // Assert
    assert_eq!(
        res.status(),
        reqwest::StatusCode::NOT_MODIFIED,
        "expected 304 Not Modified"
    );

    // 304 responses must not include a body
    let body = res.text().unwrap();
    assert!(body.is_empty(), "expected empty body for 304 response");
}

#[test]
fn if_modified_since_returns_304() {
    // Arrange
    common::start_server(&SERVER, CONFIG);

    let initial = reqwest::blocking::get("http://127.0.0.1:4041/").expect("initial request failed");

    let last_modified = initial
        .headers()
        .get(reqwest::header::LAST_MODIFIED)
        .expect("Last-Modified missing")
        .to_str()
        .unwrap()
        .to_string();

    // Act
    let client = reqwest::blocking::Client::new();
    let res = client
        .get("http://127.0.0.1:4041/")
        .header(reqwest::header::IF_MODIFIED_SINCE, last_modified)
        .send()
        .expect("conditional request failed");

    // Assert
    assert_eq!(
        res.status(),
        reqwest::StatusCode::NOT_MODIFIED,
        "expected 304 Not Modified"
    );
}
