use integration_tests::conf::minimal_https_runtime_config_with_acme;
use integration_tests::harness::TestServer;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use std::thread::sleep;
use std::time::{Duration, Instant};

/// Assumes:
/// - Pebble is already running
/// - ACME directory URL points to Pebble
/// - HTTP-01 challenge routing is wired
/// - /admin/certs endpoint exists
#[test]
fn should_issue_certificate_via_http01_and_serve_tls() {
    //-------------------------------------------------------------------------
    // Arrange
    //-------------------------------------------------------------------------
    let mut cfg = minimal_https_runtime_config_with_acme();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    //-------------------------------------------------------------------------
    // Act: wait for certificate issuance
    //-------------------------------------------------------------------------
    let admin_client = Client::builder()
        .danger_accept_invalid_certs(true) // Pebble CA
        .build()
        .unwrap();

    let timeout = Duration::from_secs(60);
    let start = Instant::now();

    loop {
        if start.elapsed() > timeout {
            panic!("timed out waiting for certificate issuance");
        }

        let resp = admin_client
            .get(format!("{}/admin/certs", srv.admin_url()))
            .send()
            .expect("admin request failed");

        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.text().unwrap();
        println!("ADMIN BODY: {}", body);
        if body.contains("Valid") {
            break;
        }
        sleep(Duration::from_millis(1000));
    }

    //-------------------------------------------------------------------------
    // Assert: verify real TLS handshake works
    //-------------------------------------------------------------------------
    let https_client = Client::builder()
        .danger_accept_invalid_certs(true) // Pebble CA
        .build()
        .expect("TLS client builder failed");

    // Use domain in URL so SNI is correct
    let res = https_client
        .get(srv.https_url())
        .send()
        .expect("TLS request failed");

    // Even though the page isn't found,
    // the fact that the request returned ANY HTTP response
    // is proof connecting with the TLS cert worked.
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
