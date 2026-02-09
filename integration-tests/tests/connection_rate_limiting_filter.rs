use integration_tests::conf::ConfigBuilder;
use integration_tests::harness::TestServer;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use std::thread::sleep;
use std::time::{Duration, Instant};

//-----------------------------------------------------------------------------
// Disabled / baseline behavior
//-----------------------------------------------------------------------------

#[test]
fn connection_rate_limiter_disabled_allows_request() {
    // Arrange
    let mut cfg = ConfigBuilder::default().with_http_ingress().build();

    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send().expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
}

//-----------------------------------------------------------------------------
// Basic allow behavior
//-----------------------------------------------------------------------------

#[test]
fn connection_rate_limiter_allows_single_connection_under_limit() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_connection_rate_limiting_filter(10, 1)
        .build();

    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send().expect("request failed");

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
}

//-----------------------------------------------------------------------------
// Sustained pressure behavior
//-----------------------------------------------------------------------------

#[test]
fn connection_rate_limiter_eventually_rejects_under_sustained_pressure() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_connection_rate_limiting_filter(3, 1)
        .build();

    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act: repeatedly create new connections
    let start = Instant::now();
    let mut saw_rejection = false;

    while start.elapsed() < Duration::from_secs(3) {
        let res = srv.get("/api").send();

        // Connection filter rejection manifests as a client-side error.
        if res.is_err() {
            saw_rejection = true;
            break;
        }

        // Small pause so time advances and interval rollover can occur.
        sleep(Duration::from_millis(20));
    }

    // Assert
    assert!(
        saw_rejection,
        "expected connection rate limiter to eventually reject under sustained pressure"
    );
}

//-----------------------------------------------------------------------------
// Recovery / liveness
//-----------------------------------------------------------------------------

#[test]
fn connection_rate_limiter_does_not_permanently_reject_after_pressure_stops() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_connection_rate_limiting_filter(3, 1)
        .build();

    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Apply sustained pressure
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        let _ = srv.get("/api").send();
        sleep(Duration::from_millis(20));
    }

    // Stop creating connections.
    sleep(Duration::from_secs(2));

    // Act
    // Attempt connections until one succeeds.
    let mut saw_success = false;
    for _ in 0..20 {
        if let Ok(res) = srv.get("/api").send() {
            if res.status() == StatusCode::OK {
                saw_success = true;
                break;
            }
        }
        sleep(Duration::from_millis(50));
    }

    // Assert
    assert!(
        saw_success,
        "expected connection rate limiter to eventually allow new connections after pressure stops"
    );
}
