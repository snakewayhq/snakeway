use confval::provenance::Located;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use snakeway_tests::conf::{ConfigBuilder, minimal_http_runtime_config};
use snakeway_tests::harness::TestServer;
use std::panic;

#[test]
fn network_policy_disabled_allows_request() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send().unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
}

#[test]
fn network_policy_allows_request_from_allowed_cidr() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_identity_device(ConfigBuilder::make_identity_device_with_trusted_proxy())
        .with_network_policy(ConfigBuilder::make_network_policy_device_spec(vec![
            "127.0.0.1/32",
        ]))
        .build();

    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send().unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
}

#[test]
fn network_policy_denies_request_from_disallowed_cidr() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_identity_device(ConfigBuilder::make_identity_device_with_trusted_proxy())
        .with_network_policy(ConfigBuilder::make_network_policy_device_spec(vec![
            "10.0.0.0/8",
        ]))
        .build();

    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send().unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[test]
fn network_policy_requires_identity_device() {
    // Act
    let result = panic::catch_unwind(|| {
        ConfigBuilder::default()
            .with_http_ingress()
            .with_network_policy(ConfigBuilder::make_network_policy_device_spec(vec![
                "127.0.0.1/32",
            ]))
            .build();
    });

    // Assert
    assert!(
        result.is_err(),
        "expected config build to panic without identity device, but it did not"
    );
}

#[test]
fn network_policy_denies_forwarded_request_when_forwarding_not_allowed() {
    // Arrange
    let mut np = ConfigBuilder::make_network_policy_device_spec(vec!["0.0.0.0/0"]);
    np.forwarding.value.allow = Located::detached(false);
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_identity_device(ConfigBuilder::make_identity_device_with_trusted_proxy())
        .with_network_policy(np)
        .build();

    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv
        .get("/api")
        .header("x-forwarded-for", "1.2.3.4")
        .send()
        .unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[test]
fn network_policy_allows_forwarded_request_when_allowed() {
    // Arrange
    let mut np = ConfigBuilder::make_network_policy_device_spec(vec!["0.0.0.0/0"]);
    np.forwarding.value.allow = Located::detached(true);
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_identity_device(ConfigBuilder::make_identity_device_with_trusted_proxy())
        .with_network_policy(np)
        .build();

    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act -- use a valid IP in the allowed CIDR range (0.0.0.0/0 allows all)
    let res = srv
        .get("/api")
        .header("x-forwarded-for", "203.0.113.50")
        .send()
        .unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
}
