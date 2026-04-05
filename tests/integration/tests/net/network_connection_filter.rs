use integration::conf::ConfigBuilder;
use integration::harness::TestServer;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use snakeway_core::testing_api::conf::types::OnNoPeerAddrSpec;

#[test]
fn should_not_be_blocked_by_connection_filter() {
    // Arrange
    let mut cfg = ConfigBuilder::default().with_http_ingress().build();

    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send().expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
}

#[test]
fn should_block_request_from_denied_cidr() {
    // Arrange
    let filter = ConfigBuilder::make_connection_filter(
        None,
        Some(&["127.0.0.1/32"]),
        true,
        true,
        OnNoPeerAddrSpec::Deny,
    );
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_connection_filter(filter)
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send();

    // Assert
    assert!(res.is_err(), "request unexpectedly succeeded");
}

#[test]
fn should_reject_ipv4_when_ipv4_is_disabled() {
    // Arrange
    let filter =
        ConfigBuilder::make_connection_filter(None, None, false, true, OnNoPeerAddrSpec::Deny);
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_connection_filter(filter)
        .build();

    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send();

    // Assert
    assert!(res.is_err(), "ipv4 request unexpectedly succeeded");
}
