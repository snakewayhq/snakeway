use confval::source::Located;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use snakeway_core::testing_api::conf::types::WasmDeviceSpec;
use snakeway_tests::conf::ConfigBuilder;
use snakeway_tests::device::{default_device, make_wasm_device};
use snakeway_tests::harness::TestServer;
use std::collections::HashMap;
use std::path::PathBuf;

/// A normal request passes through the WASM device unmodified.
#[test]
fn wasm_passthrough() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(default_device())
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send().expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
}

/// The device blocks requests to /block with 403 Forbidden.
#[test]
fn wasm_blocks_request() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(default_device())
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/block").send().expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

/// The device returns a synthetic 299 response for /synthetic.
#[test]
fn wasm_synthetic_response() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(default_device())
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/synthetic").send().expect("request failed");

    // Assert
    assert_eq!(res.status(), 299);
    assert_eq!(res.text().unwrap(), "synthetic-ok");
}

/// The device rewrites /rewrite to /api, which the upstream serves.
#[test]
fn wasm_rewrites_route_path() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(default_device())
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/rewrite").send().expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
}

/// The device reflects a config value in the response body via config-get.
#[test]
fn wasm_config_echo() {
    // Arrange
    let config = HashMap::from([("echo_value".to_string(), "hello-from-config".to_string())]);
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(make_wasm_device(config))
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/config-echo").send().expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().unwrap(), "hello-from-config");
}
