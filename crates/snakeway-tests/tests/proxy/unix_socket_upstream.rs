use confval::prelude::Located;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use snakeway::testing_api::conf::types::{
    EndpointTlsSpec, ServiceRouteSpec, ServiceSpec, SockSpec, UpstreamSpec,
};
use snakeway_tests::conf::ConfigBuilder;
use snakeway_tests::constants::{HTTP_RESPONSE_BODY, ROUTE_PATH_API, TEST_HOST};
use snakeway_tests::harness::TestServer;
use snakeway_tests::harness::server::free_port;
use snakeway_tests::harness::upstream::{start_unix_http_upstream, start_unix_https_upstream};
use std::path::{Path, PathBuf};

fn socket_service(path: &Path, tls: Option<EndpointTlsSpec>) -> ServiceSpec {
    ServiceSpec {
        name: Located::detached("api".to_string()),
        routes: vec![Located::detached(ServiceRouteSpec {
            hosts: vec![Located::detached(TEST_HOST.to_string())],
            path: Located::detached(ROUTE_PATH_API.to_string()),
            ..Default::default()
        })],
        upstreams: vec![Located::detached(UpstreamSpec {
            endpoint: None,
            sock: Some(Located::detached(SockSpec {
                path: Located::detached(path.display().to_string()),
                tls: tls.map(Located::detached),
            })),
            weight: Located::detached(1),
        })],
        ..Default::default()
    }
}

fn verifying_tls(ca_file: Option<PathBuf>) -> EndpointTlsSpec {
    EndpointTlsSpec {
        sni: Located::detached(TEST_HOST.to_string()),
        verify: Located::detached(true),
        ca_file: ca_file.map(Located::detached),
    }
}

/// A certificate made during the test, so no system trust store can hold it.
fn generate_upstream_identity() -> (String, String) {
    let generated = rcgen::generate_simple_self_signed(vec![TEST_HOST.into()])
        .expect("failed to generate upstream certificate");
    (generated.cert.pem(), generated.signing_key.serialize_pem())
}

/// A plain HTTP listener proxies to a Unix socket upstream that serves plain HTTP.
#[test]
fn plain_listener_proxies_to_plain_unix_socket_upstream() {
    // Arrange
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let socket_path = dir.path().join("plain.sock");
    start_unix_http_upstream(&socket_path);
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![socket_service(&socket_path, None)])
        .build();
    let srv = TestServer::start_with_config(&mut cfg, free_port);

    // Act
    let res = srv.get(ROUTE_PATH_API).send().expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().expect("failed to read body"), HTTP_RESPONSE_BODY);
}

/// A TLS listener proxies to a Unix socket upstream that serves plain HTTP.
/// TLS on the listener must not decide whether Snakeway uses TLS to the socket.
#[test]
fn tls_listener_proxies_to_plain_unix_socket_upstream() {
    // Arrange
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let socket_path = dir.path().join("plain.sock");
    start_unix_http_upstream(&socket_path);
    let mut cfg = ConfigBuilder::default()
        .with_custom_tls_ingress(vec![socket_service(&socket_path, None)])
        .build();
    let srv = TestServer::start_with_config(&mut cfg, free_port);
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("failed to build client");
    let url = format!("https://{}{}", srv.https_addr(), ROUTE_PATH_API);

    // Act
    let res = client.get(&url).send().expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().expect("failed to read body"), HTTP_RESPONSE_BODY);
}

/// If a Unix socket upstream has a `tls` block with its certificate in `ca_file`, Snakeway
/// connects over TLS and trusts the certificate.
#[test]
fn unix_socket_upstream_with_tls_is_trusted_through_its_ca_file() {
    // Arrange
    let (cert_pem, key_pem) = generate_upstream_identity();
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let socket_path = dir.path().join("tls.sock");
    let ca_file = dir.path().join("upstream.pem");
    std::fs::write(&ca_file, &cert_pem).expect("failed to write CA file");
    start_unix_https_upstream(&socket_path, &cert_pem, &key_pem);
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![socket_service(
            &socket_path,
            Some(verifying_tls(Some(ca_file))),
        )])
        .build();
    let srv = TestServer::start_with_config(&mut cfg, free_port);

    // Act
    let res = srv.get(ROUTE_PATH_API).send().expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().expect("failed to read body"), HTTP_RESPONSE_BODY);
}

/// If a Unix socket upstream sets `verify = true` and no CA file applies, Snakeway checks the
/// certificate against the trust store of its connector, which does not hold this certificate.
#[test]
fn unix_socket_upstream_certificate_is_verified_when_no_ca_file_is_set() {
    // Arrange
    let (cert_pem, key_pem) = generate_upstream_identity();
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let socket_path = dir.path().join("tls.sock");
    start_unix_https_upstream(&socket_path, &cert_pem, &key_pem);
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![socket_service(
            &socket_path,
            Some(verifying_tls(None)),
        )])
        .build();
    let srv = TestServer::start_with_config(&mut cfg, free_port);

    // Act
    let res = srv.get(ROUTE_PATH_API).send().expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::BAD_GATEWAY);
}
