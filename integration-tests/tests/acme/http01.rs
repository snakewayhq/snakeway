use integration_tests::conf::minimal_https_runtime_config_with_acme;
use integration_tests::harness::TestServer;
use pretty_assertions::assert_eq;
use reqwest::{Client, StatusCode};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// Assumes:
/// - Pebble is already running
/// - ACME directory URL points to Pebble
/// - HTTP-01 challenge routing is wired
/// - /admin/certs endpoint exists
#[tokio::test(flavor = "multi_thread")]
async fn should_issue_certificate_via_http01_and_serve_tls() {
    //-------------------------------------------------------------------------
    // Arrange
    //-------------------------------------------------------------------------
    let mut cfg = minimal_https_runtime_config_with_acme();

    // Domain used for ACME order.
    // Must resolve to localhost for Pebble http-01 validation.
    let domain = "snakeway.test";

    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    //-------------------------------------------------------------------------
    // Act: wait for certificate issuance
    //-------------------------------------------------------------------------
    let admin_client = Client::builder()
        .danger_accept_invalid_certs(true) // Pebble CA
        .build()
        .unwrap();

    let timeout = Duration::from_secs(30);
    let start = Instant::now();

    loop {
        if start.elapsed() > timeout {
            panic!("timed out waiting for certificate issuance");
        }

        let resp = admin_client
            .get(format!("{}/admin/certs", srv.admin_url()))
            .send()
            .await
            .expect("admin request failed");

        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.text().await.unwrap();

        if body.contains("\"state\":\"Valid\"") {
            break;
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    //-------------------------------------------------------------------------
    // Assert: verify real TLS handshake works
    //-------------------------------------------------------------------------

    let socket: SocketAddr = srv.https_addr().parse().expect("invalid listener addr");

    let https_client = Client::builder()
        .danger_accept_invalid_certs(true) // Pebble CA
        .resolve(domain, socket) // override DNS → localhost:port
        .build()
        .expect("TLS client builder failed");

    // Use domain in URL so SNI is correct
    let res = https_client
        .get(format!("https://{}/", domain))
        .send()
        .await
        .expect("TLS request failed");

    assert_eq!(res.status(), StatusCode::OK);
}
