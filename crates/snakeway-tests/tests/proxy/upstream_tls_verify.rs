use confval::prelude::Located;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use snakeway::testing_api::conf::types::{
    EndpointSpec, EndpointTlsSpec, ServiceRouteSpec, ServiceSpec, UpstreamSpec,
};
use snakeway_tests::conf::ConfigBuilder;
use snakeway_tests::constants::{
    HTTP_RESPONSE_BODY, ROUTE_PATH_API, TEST_HOST, UPSTREAM_PORT_PRIMARY,
};
use snakeway_tests::harness::TestServer;
use snakeway_tests::harness::upstream::start_https_upstream;
use std::path::PathBuf;

struct UpstreamIdentity {
    cert_pem: String,
    key_pem: String,
}

/// A certificate made during the test, so no system trust store can hold it.
fn generate_upstream_identity() -> UpstreamIdentity {
    let generated = rcgen::generate_simple_self_signed(vec![TEST_HOST.into()])
        .expect("failed to generate upstream certificate");
    UpstreamIdentity {
        cert_pem: generated.cert.pem(),
        key_pem: generated.signing_key.serialize_pem(),
    }
}

fn verifying_service(ca_file: Option<PathBuf>) -> ServiceSpec {
    ServiceSpec {
        name: Located::detached("api".to_string()),
        routes: vec![Located::detached(ServiceRouteSpec {
            hosts: vec![Located::detached(TEST_HOST.to_string())],
            path: Located::detached(ROUTE_PATH_API.to_string()),
            ..Default::default()
        })],
        upstreams: vec![Located::detached(UpstreamSpec {
            endpoint: Some(Located::detached(EndpointSpec {
                host: Located::detached(TEST_HOST.to_string()),
                port: Located::detached(UPSTREAM_PORT_PRIMARY),
                tls: Some(Located::detached(EndpointTlsSpec {
                    sni: Located::detached(TEST_HOST.to_string()),
                    verify: Located::detached(true),
                    ca_file: ca_file.map(Located::detached),
                })),
            })),
            sock: None,
            weight: Located::detached(1),
        })],
        ..Default::default()
    }
}

/// If an upstream sets `verify = true` and no CA file applies, Snakeway must still check the
/// upstream certificate against the trust store of its connector.
/// The test upstream presents a certificate that no trust store holds, so the proxy cannot connect.
#[test]
fn upstream_certificate_is_verified_when_no_ca_file_is_set() {
    // Arrange
    let identity = generate_upstream_identity();
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![verifying_service(None)])
        .build();
    let srv = TestServer::start_with_config(&mut cfg, || {
        start_https_upstream(&identity.cert_pem, &identity.key_pem)
    });

    // Act
    let res = srv.get(ROUTE_PATH_API).send().expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::BAD_GATEWAY);
}

/// If the upstream certificate is in the upstream `ca_file`, the same upstream is trusted.
#[test]
fn upstream_certificate_is_trusted_through_its_ca_file() {
    // Arrange
    let identity = generate_upstream_identity();
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let ca_file = dir.path().join("upstream.pem");
    std::fs::write(&ca_file, &identity.cert_pem).expect("failed to write CA file");
    let mut cfg = ConfigBuilder::default()
        .with_custom_ingress(vec![verifying_service(Some(ca_file))])
        .build();
    let srv = TestServer::start_with_config(&mut cfg, || {
        start_https_upstream(&identity.cert_pem, &identity.key_pem)
    });

    // Act
    let res = srv.get(ROUTE_PATH_API).send().expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().expect("failed to read body"), HTTP_RESPONSE_BODY);
}
