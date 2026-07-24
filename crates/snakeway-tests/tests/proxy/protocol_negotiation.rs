//! HTTP protocol and upgrade negotiation tests.
//!
//! These establish what Pingora and the current `TrafficProxy` actually do at
//! the protocol boundary, so the negotiation model is documented.

use snakeway_tests::conf::{minimal_h2_to_h1_runtime_config, minimal_http_runtime_config};
use snakeway_tests::constants::{ROUTE_PATH_API, ROUTE_PATH_WS, TEST_HOST};
use snakeway_tests::h2_over_tls::send_h2_request;
use snakeway_tests::harness::TestServer;
use std::time::Duration;

/// Snakeway offers HTTP/2 only over TLS (via ALPN).
/// A plaintext HTTP/1.1 listener does not speak cleartext HTTP/2 (h2c), so a
/// client attempting an h2c connection by prior knowledge must fail rather
/// than be served.
#[test]
fn h2c_prior_knowledge_on_plaintext_listener_is_rejected() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let client = reqwest::blocking::Client::builder()
        .http2_prior_knowledge()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("failed to build h2c client");
    let url = srv.base_url().join(ROUTE_PATH_API).expect("valid url");

    // Act
    let result = client.get(url).send();

    // Assert
    assert!(
        result.is_err(),
        "plaintext HTTP/1.1 listener must reject an h2c prior-knowledge client, got: {result:?}"
    );
}

/// A well-formed HTTP/2 request is proxied normally through the low-level h2
/// client harness.
/// This is the control case: it proves the harness works, so a failure in the
/// tests below is attributable to the protocol quirk, not setup.
#[test]
fn h2_over_tls_harness_proxies_a_normal_get() {
    // Arrange
    let mut cfg = minimal_h2_to_h1_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let addr = srv.https_addr().to_string();
    let req = http::Request::builder()
        .method(http::Method::GET)
        .uri(format!("https://{addr}{ROUTE_PATH_API}"))
        .header("host", TEST_HOST)
        .body(())
        .expect("request");

    // Act
    let outcome = send_h2_request(&addr, req, true);

    // Assert
    assert!(
        matches!(outcome, Ok(s) if s == http::StatusCode::OK),
        "expected a 200 over the low-level h2 harness, got {outcome:?}"
    );
}

/// HTTP/2 forbids connection-specific headers such as `Upgrade` and
/// `Connection` (RFC 9113 §8.2.2).
/// A request bearing them must not be proxied to the upstream, regardless of
/// whether the rejection surfaces as a client refusal, a stream reset, or a
/// 4xx response.
#[test]
fn classic_upgrade_header_on_http2_is_not_proxied() {
    // Arrange
    let mut cfg = minimal_h2_to_h1_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let addr = srv.https_addr().to_string();
    let req = http::Request::builder()
        .method(http::Method::GET)
        .uri(format!("https://{addr}{ROUTE_PATH_API}"))
        .header("host", TEST_HOST)
        .header("upgrade", "websocket")
        .header("connection", "upgrade")
        .body(())
        .expect("request");

    // Act
    let outcome = send_h2_request(&addr, req, true);

    // Assert
    assert!(
        !matches!(outcome, Ok(http::StatusCode::OK)),
        "an HTTP/2 request carrying a connection-specific Upgrade header must not be proxied, got {outcome:?}"
    );
}

/// WebSocket over HTTP/2 uses Extended CONNECT (RFC 8441), which requires the
/// server to advertise `SETTINGS_ENABLE_CONNECT_PROTOCOL`.
/// Snakeway is not expected to support this, so the tunnel must not be
/// established, no success status is expected or returned.
///
/// Should this test ever fail, it may mean Pingora started supporting
/// Extend CONNECT, in which case Snakeway can then support websockets
/// over HTTP/2.
#[test]
fn extended_connect_websocket_over_http2_is_not_supported() {
    // Arrange
    let mut cfg = minimal_h2_to_h1_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let addr = srv.https_addr().to_string();
    let mut req = http::Request::builder()
        .method(http::Method::CONNECT)
        .uri(format!("https://{addr}{ROUTE_PATH_WS}"))
        .header("host", TEST_HOST)
        .body(())
        .expect("request");
    req.extensions_mut()
        .insert(h2::ext::Protocol::from("websocket"));

    // Act
    let outcome = send_h2_request(&addr, req, false);

    // Assert
    assert!(
        !matches!(outcome, Ok(s) if s.is_success()),
        "WebSocket-over-HTTP/2 (Extended CONNECT) is not expected to be supported, got {outcome:?}"
    );
}
