use snakeway_tests::conf::ConfigBuilder;
use snakeway_tests::device::{device_with_mode, device_with_mode_and_hooks};
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

/// With `hooks = ["on_request"]` the after_proxy hook is skipped, so the status override
/// the "set-status" mode would apply never runs, and the upstream status passes through.
#[test]
fn wasm_hooks_allowlist_skips_after_proxy() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_wasm_device(device_with_mode_and_hooks("set-status", &["on_request"]))
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send().expect("request failed");

    // Assert
    pretty_assertions::assert_ne!(res.status(), 299);
}
