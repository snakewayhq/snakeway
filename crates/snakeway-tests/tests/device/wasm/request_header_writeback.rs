//! Request-header operations emitted by a WASM device must reach the upstream,
//! and a device-set header must win over a client-supplied header of the same name.

use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use snakeway_tests::conf::ConfigBuilder;
use snakeway_tests::device::device_with_mode;
use snakeway_tests::harness::TestServer;

/// Parse the echo-headers upstream response as JSON and look up a header by
/// name. The upstream lowercases header names.
fn echoed_header(body: &str, name: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(body)
        .unwrap_or_else(|e| panic!("upstream response is not valid JSON: {e}\nbody: {body}"));
    json.get(name).and_then(|v| v.as_str()).map(String::from)
}

/// A request header injected by a device's `before_proxy` hook reaches the upstream.
#[test]
fn before_proxy_injected_request_header_reaches_upstream() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(device_with_mode("inject"))
        .build();
    let srv = TestServer::start_http_upstream_that_echoes_headers_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send().expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert_eq!(
        echoed_header(&body, "x-before-proxy").as_deref(),
        Some("injected"),
        "device-injected request header must reach the upstream; upstream saw: {body}"
    );
}

/// When a device sets a request header, a client-supplied header of the same name
/// does not override it.
#[test]
fn client_cannot_spoof_device_managed_request_header() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(device_with_mode("inject"))
        .build();
    let srv = TestServer::start_http_upstream_that_echoes_headers_with_config(&mut cfg);

    // Act
    let res = srv
        .get("/api")
        .header("x-before-proxy", "spoofed-by-client")
        .send()
        .expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert_eq!(
        echoed_header(&body, "x-before-proxy").as_deref(),
        Some("injected"),
        "device must overwrite the client-supplied header; upstream saw: {body}"
    );
}
