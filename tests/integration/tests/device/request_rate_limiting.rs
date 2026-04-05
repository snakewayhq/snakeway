use integration::conf::{ConfigBuilder, minimal_http_runtime_config};
use integration::harness::TestServer;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use std::panic;
use std::thread::sleep;
use std::time::{Duration, Instant};

fn identity_with_trusted_proxy() -> snakeway_core::testing_api::conf::types::IdentityDeviceSpec {
    let mut id = ConfigBuilder::make_identity_device();
    id.trusted_proxies = vec!["127.0.0.1/32".to_string()];
    id
}

//-----------------------------------------------------------------------------
// Disabled / wiring tests
//-----------------------------------------------------------------------------

#[test]
fn request_rate_limit_disabled_allows_requests() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send().unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
}

#[test]
fn request_rate_limit_requires_identity_device() {
    // Act
    let result = panic::catch_unwind(|| {
        ConfigBuilder::default()
            .with_http_ingress()
            .with_request_rate_limiting(10, 1)
            .build();
    });

    // Assert
    assert!(
        result.is_err(),
        "expected config build to panic without identity device, but it did not"
    );
}

//-----------------------------------------------------------------------------
// Basic allow behavior
//-----------------------------------------------------------------------------

#[test]
fn request_rate_limit_allows_single_request_under_limit() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_identity_device(identity_with_trusted_proxy())
        .with_request_rate_limiting(10, 1)
        .build();

    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send().unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
}

//-----------------------------------------------------------------------------
// Sustained pressure behavior
//-----------------------------------------------------------------------------

#[test]
fn request_rate_limit_eventually_rejects_under_sustained_pressure() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_identity_device(identity_with_trusted_proxy())
        .with_request_rate_limiting(3, 1)
        .build();

    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act: apply pressure across time
    let start = Instant::now();
    let mut saw_rejection = false;

    while start.elapsed() < Duration::from_secs(3) {
        let res = srv.get("/api").send().unwrap();

        if res.status() == StatusCode::TOO_MANY_REQUESTS {
            saw_rejection = true;
            break;
        }

        // Small sleep so time can advance and interval rollover can happen
        sleep(Duration::from_millis(20));
    }

    // Assert
    assert!(
        saw_rejection,
        "expected request rate limiter to eventually reject under sustained pressure"
    );
}

//-----------------------------------------------------------------------------
// Recovery / liveness
//-----------------------------------------------------------------------------

#[test]
fn request_rate_limit_does_not_permanently_reject_after_pressure_stops() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_identity_device(identity_with_trusted_proxy())
        .with_request_rate_limiting(3, 1)
        .build();

    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Apply sustained pressure
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        let _ = srv.get("/api").send();
        sleep(Duration::from_millis(20));
    }

    // Stop traffic completely
    sleep(Duration::from_secs(2));

    // Act: try until allowed again
    let mut saw_allow = false;
    for _ in 0..20 {
        let res = srv.get("/api").send().unwrap();
        if res.status() == StatusCode::OK {
            saw_allow = true;
            break;
        }
        sleep(Duration::from_millis(50));
    }

    // Assert
    assert!(
        saw_allow,
        "expected request rate limiter to eventually allow after traffic stops"
    );
}
