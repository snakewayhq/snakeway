use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use snakeway_core::testing_api::conf::types::Http2Spec;
use snakeway_tests::conf::ConfigBuilder;
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

/// A configured `max_header_list_size` must be advertised to HTTP/2 clients
/// and enforced on the connection: a request whose headers exceed the limit
/// is refused with 431, while a small request on the same listener succeeds.
#[test]
fn h2_max_header_list_size_is_enforced_on_the_wire() {
    //-------------------------------------------------------------------------
    // Arrange
    //-------------------------------------------------------------------------
    let mut cfg = ConfigBuilder::default()
        .with_h2_to_h1_ingress_with_http2_options(Http2Spec {
            max_header_list_size: Some(512),
            ..Default::default()
        })
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let client = reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true)
        .http2_prior_knowledge()
        .build()
        .expect("failed to build HTTP/2 client");
    let url = format!("https://{}{}", srv.https_addr(), ROUTE_PATH_API);

    //-------------------------------------------------------------------------
    // Act + Assert: oversized headers are rejected by the advertised limit
    //-------------------------------------------------------------------------
    let oversized = client
        .get(&url)
        .header("Host", TEST_HOST)
        .header("x-large-header", "x".repeat(4096))
        .send()
        .expect("request failed");
    assert_eq!(
        oversized.status(),
        StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE
    );

    //-------------------------------------------------------------------------
    // Act + Assert: a request within the limit still proxies normally
    //-------------------------------------------------------------------------
    let res = client
        .get(&url)
        .header("Host", TEST_HOST)
        .send()
        .expect("small request failed");
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().unwrap(), HTTP_RESPONSE_BODY);
}
