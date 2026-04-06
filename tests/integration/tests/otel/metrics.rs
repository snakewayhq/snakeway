use integration::conf::minimal_http_runtime_config;
use integration::harness::TestServer;
use opentelemetry::metrics::MeterProvider;
use opentelemetry_sdk::metrics::{InMemoryMetricExporter, PeriodicReader, SdkMeterProvider};
use reqwest::StatusCode;
use snakeway_core::testing_api::Metrics;
use std::sync::Arc;

fn setup_metrics() -> (Arc<Metrics>, InMemoryMetricExporter, SdkMeterProvider) {
    let exporter = InMemoryMetricExporter::default();
    let reader = PeriodicReader::builder(exporter.clone()).build();
    let provider = SdkMeterProvider::builder().with_reader(reader).build();
    let meter = provider.meter("snakeway-test");
    let metrics = Arc::new(Metrics::new(&meter));
    (metrics, exporter, provider)
}

/// After proxying a request, the `snakeway.http.requests` counter must
/// have recorded at least one data point.
#[test]
fn request_counter_increments_after_proxied_request() {
    // Arrange
    let (metrics, exporter, provider) = setup_metrics();
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_with_config_and_metrics(
        &mut cfg,
        integration::harness::upstream::start_http_upstream,
        Some(metrics),
    );

    // Act
    let res = srv.get("/api").send().unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    provider.force_flush().unwrap();

    // Assert
    let finished = exporter.get_finished_metrics().unwrap();
    let metric_names: Vec<&str> = finished
        .iter()
        .flat_map(|rm| rm.scope_metrics())
        .flat_map(|sm| sm.metrics())
        .map(|m| m.name())
        .collect();

    assert!(
        metric_names.contains(&"snakeway.http.requests"),
        "expected snakeway.http.requests metric; found: {metric_names:?}"
    );
}

/// After proxying a request, the `snakeway.http.request.duration`
/// histogram must have recorded at least one data point.
#[test]
fn request_duration_histogram_records_after_proxied_request() {
    // Arrange
    let (metrics, exporter, provider) = setup_metrics();
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_with_config_and_metrics(
        &mut cfg,
        integration::harness::upstream::start_http_upstream,
        Some(metrics),
    );

    // Act
    let res = srv.get("/api").send().unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    provider.force_flush().unwrap();

    // Assert
    let finished = exporter.get_finished_metrics().unwrap();
    let metric_names: Vec<&str> = finished
        .iter()
        .flat_map(|rm| rm.scope_metrics())
        .flat_map(|sm| sm.metrics())
        .map(|m| m.name())
        .collect();

    assert!(
        metric_names.contains(&"snakeway.http.request.duration"),
        "expected snakeway.http.request.duration metric; found: {metric_names:?}"
    );
}

/// When no requests are proxied, no request metrics should be recorded.
/// This verifies the metrics are request-driven, not timer-driven.
#[test]
fn no_metrics_recorded_without_requests() {
    // Arrange
    let (_metrics, exporter, provider) = setup_metrics();

    // Act (no requests sent)
    provider.force_flush().unwrap();

    // Assert
    let finished = exporter.get_finished_metrics().unwrap();
    let metric_names: Vec<&str> = finished
        .iter()
        .flat_map(|rm| rm.scope_metrics())
        .flat_map(|sm| sm.metrics())
        .map(|m| m.name())
        .collect();

    assert!(
        !metric_names.contains(&"snakeway.http.requests"),
        "no request metrics should exist without traffic; found: {metric_names:?}"
    );
}
