use crate::server::UpstreamId;
use crate::traffic::circuit::*;
use crate::traffic::types::ServiceId;
use std::time::Duration;

fn params() -> CircuitBreakerParams {
    CircuitBreakerParams {
        enabled: true,
        failure_threshold: 3,
        open_duration: Duration::from_millis(100),
        half_open_max_requests: 1,
        success_threshold: 2,
        count_http_5xx_as_failure: true,
    }
}

fn ids() -> (ServiceId, UpstreamId) {
    (ServiceId("test".into()), UpstreamId(1))
}

#[test]
fn test_cb_trip_open() {
    let mut cb = CircuitBreaker::new();
    let p = params();
    let ids = ids();
    let ids_ref = (&ids.0, &ids.1);

    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.allow_request(ids_ref, &p));

    // 1 failure
    cb.on_request_end(ids_ref, &p, true, false);
    assert_eq!(cb.state(), CircuitState::Closed);

    // 2 failures
    cb.on_request_end(ids_ref, &p, true, false);
    assert_eq!(cb.state(), CircuitState::Closed);

    // 3 failures -> Open
    cb.on_request_end(ids_ref, &p, true, false);
    assert_eq!(cb.state(), CircuitState::Open);
    assert!(!cb.allow_request(ids_ref, &p));
}

#[test]
fn test_cb_cooldown_to_half_open() {
    let mut cb = CircuitBreaker::new();
    let p = params();
    let ids = ids();
    let ids_ref = (&ids.0, &ids.1);

    // Trip it
    cb.on_request_end(ids_ref, &p, true, false);
    cb.on_request_end(ids_ref, &p, true, false);
    cb.on_request_end(ids_ref, &p, true, false);
    assert_eq!(cb.state(), CircuitState::Open);

    // Immediate check - still open
    assert!(!cb.allow_request(ids_ref, &p));

    // Wait for cooldown
    std::thread::sleep(Duration::from_millis(110));

    // Should allow one probe and become half-open
    assert!(cb.allow_request(ids_ref, &p));
    assert_eq!(cb.state(), CircuitState::HalfOpen);

    // Second concurrent probe should be denied
    assert!(!cb.allow_request(ids_ref, &p));
}

#[test]
fn test_cb_half_open_to_closed() {
    let mut cb = CircuitBreaker::new();
    let p = params();
    let ids = ids();
    let ids_ref = (&ids.0, &ids.1);

    // Trip and cooldown
    cb.on_request_end(ids_ref, &p, true, false);
    cb.on_request_end(ids_ref, &p, true, false);
    cb.on_request_end(ids_ref, &p, true, false);
    std::thread::sleep(Duration::from_millis(110));

    // Probe 1
    assert!(cb.allow_request(ids_ref, &p));
    cb.on_request_end(ids_ref, &p, true, true);
    assert_eq!(cb.state(), CircuitState::HalfOpen);

    // Probe 2
    assert!(cb.allow_request(ids_ref, &p));
    cb.on_request_end(ids_ref, &p, true, true);
    assert_eq!(cb.state(), CircuitState::Closed);
}

#[test]
fn test_cb_half_open_failure_reopens() {
    let mut cb = CircuitBreaker::new();
    let p = params();
    let ids = ids();
    let ids_ref = (&ids.0, &ids.1);

    // Trip and cooldown
    cb.on_request_end(ids_ref, &p, true, false);
    cb.on_request_end(ids_ref, &p, true, false);
    cb.on_request_end(ids_ref, &p, true, false);
    std::thread::sleep(Duration::from_millis(110));

    // Probe 1 fails
    assert!(cb.allow_request(ids_ref, &p));
    cb.on_request_end(ids_ref, &p, true, false);
    assert_eq!(cb.state(), CircuitState::Open);
    assert!(!cb.allow_request(ids_ref, &p));
}

#[test]
fn test_cb_disabled() {
    let mut cb = CircuitBreaker::new();
    let mut p = params();
    p.enabled = false;
    let ids = ids();
    let ids_ref = (&ids.0, &ids.1);

    // Failures shouldn't trip it
    cb.on_request_end(ids_ref, &p, true, false);
    cb.on_request_end(ids_ref, &p, true, false);
    cb.on_request_end(ids_ref, &p, true, false);
    cb.on_request_end(ids_ref, &p, true, false);
    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.allow_request(ids_ref, &p));
}
