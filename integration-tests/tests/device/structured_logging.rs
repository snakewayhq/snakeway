use integration_tests::conf::ConfigBuilder;
use integration_tests::harness::TestServer;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use std::thread::sleep;
use std::time::{Duration, Instant};

/// Structured logging device does not break proxying.
#[test]
fn structured_logging_proxies_request() {
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_structured_logging_device()
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv.get("/api").send().unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

/// Structured logging device with otel_metrics enabled does not break proxying.
#[test]
fn structured_logging_with_otel_metrics_proxies_request() {
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_structured_logging_device_and_otel_metrics()
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    let res = srv.get("/api").send().unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

/// End-to-end: a request processed by the structured logging device with
/// otel_metrics=true must produce a trace visible in Jaeger.
///
/// Requires the docker-compose-dev.yml Jaeger service to be running (port 16686
/// for the HTTP query API, port 4317 for OTLP gRPC ingestion). In CI both are
/// guaranteed by the "Wait for Jaeger" workflow step.
#[test]
fn structured_logging_otel_metrics_exports_trace_to_jaeger() {
    let mut cfg = ConfigBuilder::default()
        .with_http_ingress()
        .with_structured_logging_device_otel_metrics_and_endpoint(
            "http://localhost:4317",
            "snakeway-integration-test",
        )
        .build();
    let srv = TestServer::start_http_upstream_with_config(&mut cfg);

    // Send a request; the device creates an OTel span for it.
    let res = srv.get("/otel-test").send().unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Give the OTel SDK's background batch exporter time to flush.
    sleep(Duration::from_secs(3));

    // Poll the Jaeger HTTP query API until the trace appears or we time out.
    let jaeger_url = "http://localhost:16686/api/traces?service=snakeway-integration-test&limit=1";
    let client = reqwest::blocking::Client::new();
    let deadline = Instant::now() + Duration::from_secs(15);

    loop {
        if let Ok(resp) = client.get(jaeger_url).send() {
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().unwrap_or_default();
                let traces = body
                    .get("data")
                    .and_then(|d| d.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);

                if traces > 0 {
                    let span_name = body["data"][0]["spans"][0]["operationName"]
                        .as_str()
                        .unwrap_or("");
                    assert_eq!(
                        span_name, "http.request",
                        "expected span name 'http.request', got '{span_name}'"
                    );
                    return;
                }
            }
        }

        assert!(
            Instant::now() < deadline,
            "Jaeger did not receive a trace for service 'snakeway-integration-test' \
             within 15 seconds. Is Jaeger running on localhost:4317/16686?"
        );

        sleep(Duration::from_secs(2));
    }
}
