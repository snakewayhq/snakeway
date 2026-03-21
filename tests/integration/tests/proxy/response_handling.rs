use integration::conf::minimal_http_runtime_config;
use integration::constants::{HTTP_RESPONSE_BODY, ROUTE_PATH_API};
use integration::harness::TestServer;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;

//-----------------------------------------------------------------------------
// Response body passthrough
//-----------------------------------------------------------------------------

/// The proxy must forward the upstream's response body to the client intact.
///
/// The upstream mock always returns the fixed string `HTTP_RESPONSE_BODY`
/// ("hello world"). Verifying the body — not just the status code — confirms
/// the proxy is not stripping, truncating, or substituting the body.
#[test]
fn response_body_is_proxied() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get(ROUTE_PATH_API).send().unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().unwrap(), HTTP_RESPONSE_BODY);
}

/// The proxy must forward the upstream's response body on POST requests.
///
/// Some proxy implementations handle GET response bodies correctly but
/// suppress or discard response bodies when the request method is POST.
#[test]
fn post_response_body_is_proxied() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.post(ROUTE_PATH_API).send().unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().unwrap(), HTTP_RESPONSE_BODY);
}

/// The proxy must forward the upstream's response on DELETE requests.
///
/// DELETE is semantically different from GET/POST but the proxy must not
/// silently drop or alter the response body returned by the upstream.
#[test]
fn delete_response_is_proxied() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.delete(ROUTE_PATH_API).send().unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().unwrap(), HTTP_RESPONSE_BODY);
}

//-----------------------------------------------------------------------------
// Routing: unmatched requests
//-----------------------------------------------------------------------------

/// A request whose path does not match any configured route must receive
/// a 404 Not Found response from the proxy.
///
/// A missing route is a proxy-level decision (the request never reaches
/// an upstream). This test confirms Snakeway's routing logic rejects
/// unmatched paths cleanly rather than forwarding blindly or panicking.
#[test]
fn unmatched_path_returns_404() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/nonexistent").send().unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

/// A request carrying a Host header that does not match any configured
/// virtual host must receive a 404 Not Found response.
///
/// Host-based routing is a core reverse proxy feature. Sending to an
/// unlisted virtual host must not accidentally match a catch-all route
/// or forward to an upstream not associated with that host.
#[test]
fn unmatched_host_returns_404() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act — override the Host header set by srv.get()
    let url = srv.base_url().join(ROUTE_PATH_API).unwrap();
    let res = srv
        .client
        .get(url)
        .header("Host", "unknown.example.com")
        .send()
        .unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

//-----------------------------------------------------------------------------
// Response headers
//-----------------------------------------------------------------------------

/// The upstream response must include a Content-Length header that the
/// proxy passes through to the client.
///
/// A missing Content-Length forces clients to buffer the entire response
/// before processing it, breaking streaming use cases and HTTP/1.1
/// keep-alive framing.
#[test]
fn response_includes_content_length_header() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get(ROUTE_PATH_API).send().unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    assert!(
        res.headers().contains_key(reqwest::header::CONTENT_LENGTH),
        "Content-Length header must be present in proxied response"
    );
}
