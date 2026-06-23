use snakeway_tests::conf::ConfigBuilder;
use snakeway_tests::device::device_with_mode;
use snakeway_tests::harness::TestServer;

/// The device overrides the upstream status code to 299 in after_proxy.
#[test]
fn wasm_after_proxy_overrides_status() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(device_with_mode("set-status"))
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send().expect("request failed");

    // Assert
    pretty_assertions::assert_eq!(res.status(), 299);
}
