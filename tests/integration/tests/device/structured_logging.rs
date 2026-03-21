use integration::conf::ConfigBuilder;
use integration::harness::TestServer;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;

//-----------------------------------------------------------------------------
// Baseline: structured logging must not interfere with proxying
//-----------------------------------------------------------------------------

/// With the structured logging device enabled, a normal GET must still
/// succeed with 200 OK.
///
/// Structured logging is a pass-through device — it observes and records
/// request/response data but must never alter the response or block
/// legitimate traffic.
#[test]
fn structured_logging_does_not_prevent_proxying() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_structured_logging_device()
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send().unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
}

//-----------------------------------------------------------------------------
// Structured logging + identity device
//-----------------------------------------------------------------------------

/// Structured logging configured with `include_identity: true` requires
/// the identity device to have run first in the pipeline.
///
/// With both devices present, a normal request must still succeed.
/// This test catches ordering bugs where the logging device tries to
/// read identity fields before they have been populated.
#[test]
fn structured_logging_with_identity_device_does_not_prevent_proxying() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_identity_device_and_no_geo()
        .with_structured_logging_device()
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv
        .get("/api")
        .header(
            "user-agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
        )
        .send()
        .unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
}

//-----------------------------------------------------------------------------
// Structured logging + request filter
//-----------------------------------------------------------------------------

/// With structured logging, identity, and request filter all active,
/// an allowed request must still reach the upstream.
///
/// This three-device pipeline is the most realistic production setup.
/// A bug in any device's interaction with the logging device could
/// break the pipeline silently.
#[test]
fn structured_logging_combined_with_request_filter_does_not_prevent_proxying() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_identity_device_and_no_geo()
        .with_structured_logging_device()
        .with_request_filter_device()
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act
    let res = srv.get("/api").send().unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
}

/// When the request filter rejects a request, the structured logging
/// device must not panic or cause the server to crash.
///
/// Devices are invoked in phases (request, before-proxy, after-proxy,
/// response). A rejected request may skip some phases.  The logging
/// device must handle phase skipping gracefully.
#[test]
fn structured_logging_on_rejected_request_does_not_crash() {
    // Arrange
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_identity_device_and_no_geo()
        .with_structured_logging_device()
        .with_request_filter_device_that_denies_get_method()
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Act — GET is denied by the request filter
    let res = srv.get("/api").send().unwrap();

    // Assert — correct rejection, no crash
    assert_eq!(
        res.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "request filter must still reject GET even with structured logging enabled"
    );
}
