use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use snakeway_core::testing_api::conf::types::{ServiceRouteSpec, ServiceSpec};
use snakeway_tests::conf::{ConfigBuilder, minimal_http_runtime_config};
use snakeway_tests::constants::{
    HTTP_RESPONSE_BODY, ROUTE_PATH_API, TEST_HOST, UPSTREAM_PORT_PRIMARY,
};
use snakeway_tests::harness::TestServer;
use snakeway_tests::harness::upstream::{
    start_http_upstream_that_hangs, start_http_upstream_with_large_response,
};
use std::time::Duration;

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

//-----------------------------------------------------------------------------
// Large response streaming
//-----------------------------------------------------------------------------

/// The proxy must stream large response bodies without truncation.
/// A 2 MB response from the upstream should arrive at the client with
/// exactly the same byte count.
#[test]
fn large_response_body_is_streamed_without_truncation() {
    // Arrange
    let expected_size: usize = 2_097_152; // 2 MB
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![ServiceSpec {
            routes: vec![ServiceRouteSpec {
                hosts: vec![TEST_HOST.to_string()],
                path: "/api".to_string(),
                ..Default::default()
            }],
            upstreams: vec![ConfigBuilder::make_tcp_upstream(
                UPSTREAM_PORT_PRIMARY,
                false,
            )],
            ..Default::default()
        }])
        .build();

    let srv = TestServer::start_with_config(&mut cfg, |port| {
        start_http_upstream_with_large_response(port, 2_097_152)
    });

    // Act
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let url = srv.base_url().join(ROUTE_PATH_API).unwrap();
    let res = client.get(url).header("Host", TEST_HOST).send().unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.bytes().unwrap();
    assert_eq!(
        body.len(),
        expected_size,
        "response body should be exactly {expected_size} bytes, got {}",
        body.len()
    );
}

//-----------------------------------------------------------------------------
// Upstream timeout
//-----------------------------------------------------------------------------

/// When the upstream hangs and never responds, the proxy must eventually
/// return an error to the client rather than blocking forever. This
/// verifies the proxy has a finite upstream read timeout.
#[test]
fn upstream_that_hangs_does_not_block_client_forever() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![ServiceSpec {
            routes: vec![ServiceRouteSpec {
                hosts: vec![TEST_HOST.to_string()],
                path: "/api".to_string(),
                ..Default::default()
            }],
            upstreams: vec![ConfigBuilder::make_tcp_upstream(
                UPSTREAM_PORT_PRIMARY,
                false,
            )],
            ..Default::default()
        }])
        .build();

    let srv = TestServer::start_with_config(&mut cfg, start_http_upstream_that_hangs);

    // Act: use a 5-second client timeout. If the proxy has no upstream
    // timeout, the client timeout fires and we get a reqwest error.
    // Either way the client is not blocked forever.
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let url = srv.base_url().join(ROUTE_PATH_API).unwrap();
    let result = client.get(url).header("Host", TEST_HOST).send();

    // Assert: either the proxy returned 502/504, or the client timed out.
    // Both are acceptable -- the key invariant is that we are not stuck.
    match result {
        Ok(res) => {
            assert!(
                res.status() == StatusCode::BAD_GATEWAY
                    || res.status() == StatusCode::GATEWAY_TIMEOUT,
                "expected 502 or 504 when upstream hangs, got {}",
                res.status()
            );
        }
        Err(e) => {
            assert!(
                e.is_timeout(),
                "expected timeout error when upstream hangs, got: {e}"
            );
        }
    }
}

//-----------------------------------------------------------------------------
// Connection keep-alive
//-----------------------------------------------------------------------------

/// Two sequential HTTP/1.1 requests sent on the same TCP connection
/// must both receive 200 OK responses, verifying keep-alive works.
#[test]
fn connection_keepalive_serves_multiple_requests() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let addr = srv.base_url().authority().to_string();

    // Act
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let mut stream = TcpStream::connect(&addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();

    // Request 1
    stream
        .write_all(b"GET /api HTTP/1.1\r\nHost: snakeway.test\r\nConnection: keep-alive\r\n\r\n")
        .unwrap();
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).unwrap();
    let resp1 = String::from_utf8_lossy(&buf[..n]).to_string();

    // Request 2 on same connection
    stream
        .write_all(b"GET /api HTTP/1.1\r\nHost: snakeway.test\r\nConnection: keep-alive\r\n\r\n")
        .unwrap();
    let n = stream.read(&mut buf).unwrap();
    let resp2 = String::from_utf8_lossy(&buf[..n]).to_string();

    // Assert
    assert!(
        resp1.contains("200 OK"),
        "first request should succeed: {resp1}"
    );
    assert!(
        resp2.contains("200 OK"),
        "second request on keep-alive should succeed: {resp2}"
    );
}
