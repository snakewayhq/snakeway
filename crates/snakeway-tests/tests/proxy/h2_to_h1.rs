use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use snakeway_tests::conf::minimal_h2_to_h1_runtime_config;
use snakeway_tests::constants::{HTTP_RESPONSE_BODY, ROUTE_PATH_API, TEST_HOST};
use snakeway_tests::harness::TestServer;

/// An HTTP/2 client connecting to a TLS listener should successfully proxy
/// to a plaintext HTTP/1.1 upstream. The proxy translates between protocols.
#[test]
fn h2_to_h1_proxy_returns_upstream_response() {
    let mut cfg = minimal_h2_to_h1_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let client = reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true)
        .http2_prior_knowledge()
        .build()
        .expect("failed to build HTTP/2 client");

    let url = format!("https://{}{}", srv.https_addr(), ROUTE_PATH_API);
    let res = client
        .get(&url)
        .header("Host", TEST_HOST)
        .send()
        .expect("request failed");

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().unwrap(), HTTP_RESPONSE_BODY);
}
