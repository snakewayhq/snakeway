use integration::conf::minimal_http_runtime_config;
use integration::harness::TestServer;
use reqwest::StatusCode;

/// When a client sends a request with a `traceparent` header, the proxy
/// must forward it to the upstream unchanged. This is the fundamental
/// requirement for a proxy to participate in distributed tracing: it must
/// not strip W3C Trace Context headers.
#[test]
fn traceparent_header_is_passed_through_to_upstream() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_that_echoes_headers_with_config(&mut cfg);
    let traceparent = "00-abcdef1234567890abcdef1234567890-1234567890abcdef-01";

    // Act
    let res = srv
        .get("/api")
        .header("traceparent", traceparent)
        .send()
        .unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert!(
        body.contains(traceparent),
        "upstream must receive the traceparent header; got body: {body}"
    );
}

/// The `tracestate` header carries vendor-specific trace data and must
/// also be forwarded to the upstream alongside `traceparent`.
#[test]
fn tracestate_header_is_passed_through_to_upstream() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_that_echoes_headers_with_config(&mut cfg);
    let traceparent = "00-abcdef1234567890abcdef1234567890-1234567890abcdef-01";
    let tracestate = "vendor1=value1,vendor2=value2";

    // Act
    let res = srv
        .get("/api")
        .header("traceparent", traceparent)
        .header("tracestate", tracestate)
        .send()
        .unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert!(
        body.contains("tracestate"),
        "upstream must receive the tracestate header; got body: {body}"
    );
}

/// When no `traceparent` header is sent by the client and OTel is not
/// configured in the test harness, the proxy should not inject one.
/// This documents the baseline behavior: trace context injection only
/// occurs when the OTel subsystem is initialized.
#[test]
fn traceparent_not_injected_when_otel_not_configured() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_that_echoes_headers_with_config(&mut cfg);

    // Act (no traceparent header sent)
    let res = srv.get("/api").send().unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert!(
        !body.contains("traceparent"),
        "proxy should not inject traceparent when OTel is not configured; got body: {body}"
    );
}
