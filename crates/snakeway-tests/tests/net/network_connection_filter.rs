use confval::source::Located;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use snakeway_server::testing_api::conf::types::{
    CidrSpec, IpFamilySpec, NetworkConnectionFilterSpec,
};
use snakeway_tests::conf::ConfigBuilder;
use snakeway_tests::harness::TestServer;

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
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_connection_filter(NetworkConnectionFilterSpec {
            cidr: Located::detached(CidrSpec {
                allow: vec![],
                deny: vec![Located::detached("127.0.0.1/32".to_string())],
            }),
            ip_family: Located::detached(IpFamilySpec {
                ipv4: Located::detached(true),
                ipv6: Located::detached(true),
            }),
            on_no_peer_addr: Located::detached("deny".to_string()),
        })
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
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_connection_filter(NetworkConnectionFilterSpec {
            cidr: Located::detached(CidrSpec::default()),
            ip_family: Located::detached(IpFamilySpec {
                ipv4: Located::detached(false),
                ipv6: Located::detached(true),
            }),
            on_no_peer_addr: Located::detached("deny".to_string()),
        })
        .build();

    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send();

    // Assert
    assert!(res.is_err(), "ipv4 request unexpectedly succeeded");
}
