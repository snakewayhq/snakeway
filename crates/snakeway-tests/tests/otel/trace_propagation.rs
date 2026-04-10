use reqwest::StatusCode;
use snakeway_tests::conf::minimal_http_runtime_config;
use snakeway_tests::harness::TestServer;

/// Parse the echo-headers upstream response as JSON and look up a header
/// by name (lowercased keys).
fn get_echoed_header(body: &str, name: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(body)
        .unwrap_or_else(|e| panic!("upstream response is not valid JSON: {e}\nbody: {body}"));
    json.get(name).and_then(|v| v.as_str()).map(String::from)
}

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
    let echoed = get_echoed_header(&body, "traceparent");
    assert_eq!(
        echoed.as_deref(),
        Some(traceparent),
        "upstream must receive the exact traceparent header; got: {echoed:?}"
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
    let echoed = get_echoed_header(&body, "tracestate");
    assert_eq!(
        echoed.as_deref(),
        Some(tracestate),
        "upstream must receive the exact tracestate header; got: {echoed:?}"
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
        get_echoed_header(&body, "traceparent").is_none(),
        "proxy should not inject traceparent when OTel is not configured; got body: {body}"
    );
}

/// A malformed `traceparent` header must be forwarded to the upstream
/// unchanged. The proxy must not strip, reject, or attempt to parse
/// trace context headers - it is a transport layer, not a validator.
#[test]
fn invalid_traceparent_format_is_passed_through() {
    // Arrange
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_http_upstream_that_echoes_headers_with_config(&mut cfg);
    let invalid_traceparent = "not-a-valid-traceparent";

    // Act
    let res = srv
        .get("/api")
        .header("traceparent", invalid_traceparent)
        .send()
        .unwrap();

    // Assert
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    let echoed = get_echoed_header(&body, "traceparent");
    assert_eq!(
        echoed.as_deref(),
        Some(invalid_traceparent),
        "proxy must pass through malformed traceparent unchanged; got: {echoed:?}"
    );
}
