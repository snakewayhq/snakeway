use http::StatusCode;
use snakeway_tests::conf::ConfigBuilder;
use snakeway_tests::device::default_device;
use snakeway_tests::harness::TestServer;

/// A body containing "BLOCK_BODY" is blocked by the device.
#[test]
fn wasm_blocks_dangerous_body() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(default_device())
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv
        .post("/api")
        .body("payload with BLOCK_BODY marker")
        .send()
        .expect("request failed");

    // Assert
    pretty_assertions::assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

/// A safe request body passes through the device.
#[test]
fn wasm_safe_body_passes() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(default_device())
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv
        .post("/api")
        .body("safe content")
        .send()
        .expect("request failed");

    // Assert
    pretty_assertions::assert_eq!(res.status(), StatusCode::OK);
}
