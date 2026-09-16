use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::header::HOST;
use snakeway_tests::conf::minimal_h2_to_h1_runtime_config;
use snakeway_tests::constants::ROUTE_PATH_API;
use snakeway_tests::harness::TestServer;

/// When a TLS client sends an SNI, the Host header must name the same host.
/// The client sends `snakeway.test` as the SNI because the listener address uses that name.
/// The request carries a different Host header.
#[test]
fn host_header_that_differs_from_sni_is_rejected() {
    // Arrange
    let mut cfg = minimal_h2_to_h1_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .http1_only()
        .build()
        .expect("failed to build client");
    let url = format!("https://{}{}", srv.https_addr(), ROUTE_PATH_API);

    // Act
    let res = client
        .get(&url)
        .header(HOST, "other.test")
        .send()
        .expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}
