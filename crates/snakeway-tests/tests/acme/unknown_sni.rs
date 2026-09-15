use reqwest::blocking::Client;
use snakeway_tests::conf::minimal_https_runtime_config_with_acme;
use snakeway_tests::harness::TestServer;
use std::net::SocketAddr;

/// If a client sends an SNI that has no certificate, the ACME listener has nothing to present.
/// The TLS handshake then fails before any HTTP request is sent.
#[test]
fn acme_listener_fails_handshake_for_sni_without_certificate() {
    // Arrange
    let mut cfg = minimal_https_runtime_config_with_acme();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let (_, port) = srv
        .https_addr()
        .rsplit_once(':')
        .expect("listener address must be host:port");
    let addr: SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .expect("listener port must form a socket address");
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .resolve("unknown.test", addr)
        .build()
        .expect("failed to build client");

    // Act
    let result = client.get(format!("https://unknown.test:{port}/")).send();

    // Assert
    let err = result.expect_err("a handshake with an SNI that has no certificate must fail");
    assert!(err.is_connect(), "expected a connect error, got: {err:?}");
    assert!(
        format!("{err:?}").contains("AlertReceived(AccessDenied)"),
        "expected the server to abort the handshake with an access denied alert, got: {err:?}"
    );
}
