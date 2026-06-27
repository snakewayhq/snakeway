use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use snakeway_tests::conf::ConfigBuilder;
use snakeway_tests::device::device_with_mode;
use snakeway_tests::harness::TestServer;

/// The device tags the response with a custom header in on_response.
#[test]
fn wasm_on_response_tags_header() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(device_with_mode("tag-response"))
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send().expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get("x-wasm-response")
            .map(|v| v.to_str().unwrap()),
        Some("tagged")
    );
}
