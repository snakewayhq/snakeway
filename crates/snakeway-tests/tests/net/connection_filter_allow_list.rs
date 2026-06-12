use confval::provenance::Located;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use snakeway_core::testing_api::conf::types::{
    CidrSpec, IpFamilySpec, NetworkConnectionFilterSpec,
};
use snakeway_tests::conf::ConfigBuilder;
use snakeway_tests::harness::TestServer;

/// The `NetworkConnectionFilterSpec.cidr.allow` field specifies a set of
/// CIDR ranges whose connections are permitted at the TCP layer - before
/// any HTTP parsing occurs.  All other connections are dropped.
///
/// This is distinct from the device-level `NetworkPolicyDevice` CIDR
/// allow-list, which runs after HTTP parsing.  The connection filter
/// operates on the raw TCP connection and requires no identity device.

//-----------------------------------------------------------------------------
// CIDR allow-list: matching IP
//-----------------------------------------------------------------------------

/// When the CIDR allow-list contains 127.0.0.1/32 (loopback), a
/// connection from localhost must succeed.
///
/// The integration test client always connects from 127.0.0.1, so this
/// configuration should always allow the test connection through.
#[test]
fn cidr_allow_list_permits_matching_ip() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_connection_filter(NetworkConnectionFilterSpec {
            cidr: Located::detached(CidrSpec {
                allow: vec![Located::detached("127.0.0.1/32".to_string())],
                deny: vec![],
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
    let res = srv.get("/api").send().expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
}

//-----------------------------------------------------------------------------
// CIDR allow-list: non-matching IP
//-----------------------------------------------------------------------------

/// When the CIDR allow-list contains only a range that does NOT include
/// 127.0.0.1 (e.g. 10.0.0.0/8), connections from localhost must be
/// dropped at the TCP layer.
///
/// The client-side error (connection refused / reset) is expected - the
/// proxy drops the connection before sending any HTTP response.
#[test]
fn cidr_allow_list_blocks_non_matching_ip() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_connection_filter(NetworkConnectionFilterSpec {
            cidr: Located::detached(CidrSpec {
                allow: vec![Located::detached("10.0.0.0/8".to_string())],
                deny: vec![],
            }),
            ip_family: Located::detached(IpFamilySpec {
                ipv4: Located::detached(true),
                ipv6: Located::detached(true),
            }),
            on_no_peer_addr: Located::detached("deny".to_string()),
        })
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act -- 127.0.0.1 is not in 10.0.0.0/8, connection must be rejected
    let res = srv.get("/api").send();

    // Assert -- connection-level rejection appears as a client error
    assert!(
        res.is_err(),
        "connection from 127.0.0.1 must be rejected when only 10.0.0.0/8 is in the allow list"
    );
}

//-----------------------------------------------------------------------------
// Deny-list takes precedence over allow-list
//-----------------------------------------------------------------------------

/// When 127.0.0.1/32 appears in BOTH the allow-list and the deny-list,
/// the deny-list must take precedence and the connection must be dropped.
///
/// Deny rules should always be the final word. An operator who explicitly
/// blocks an address should not have that rule silently overridden by the
/// presence of the address in an allow-list.
#[test]
fn cidr_allow_and_deny_deny_takes_precedence() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_connection_filter(NetworkConnectionFilterSpec {
            cidr: Located::detached(CidrSpec {
                allow: vec![Located::detached("127.0.0.1/32".to_string())],
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

    // Assert -- deny-list wins; connection is dropped
    assert!(
        res.is_err(),
        "deny-list must take precedence over allow-list for the same CIDR"
    );
}
