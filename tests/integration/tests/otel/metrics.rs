use integration::conf::minimal_http_runtime_config;
use integration::harness::TestServer;
use opentelemetry::metrics::MeterProvider;
use opentelemetry_sdk::metrics::data::{AggregatedMetrics, Metric, MetricData};
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

/// Flush and collect all finished metrics, then find the named metric
/// and apply the assertion function to it.
fn assert_metric(
    exporter: &InMemoryMetricExporter,
    provider: &SdkMeterProvider,
    name: &str,
    assertion: impl FnOnce(Option<&Metric>),
) {
    provider.force_flush().unwrap();
    let finished = exporter.get_finished_metrics().unwrap();
    let found = finished
        .iter()
        .flat_map(|rm| rm.scope_metrics())
        .flat_map(|sm| sm.metrics())
        .find(|m| m.name() == name);
    assertion(found);
}

/// After proxying a request, the `snakeway.http.requests` counter must
/// have recorded at least one data point with a value >= 1.
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

    // Assert
    assert_metric(&exporter, &provider, "snakeway.http.requests", |metric| {
        let metric = metric.expect("snakeway.http.requests metric not found");
        match metric.data() {
            AggregatedMetrics::U64(MetricData::Sum(sum)) => {
                let total: u64 = sum.data_points().map(|dp| dp.value()).sum();
                assert!(
                    total >= 1,
                    "expected snakeway.http.requests counter >= 1, got {total}"
                );
            }
            other => panic!("expected U64 Sum for snakeway.http.requests, got: {other:?}"),
        }
    });
}

/// After proxying a request, the `snakeway.http.request.duration`
/// histogram must have recorded at least one data point with count >= 1.
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

    // Assert
    assert_metric(
        &exporter,
        &provider,
        "snakeway.http.request.duration",
        |metric| {
            let metric = metric.expect("snakeway.http.request.duration metric not found");
            match metric.data() {
                AggregatedMetrics::F64(MetricData::Histogram(hist)) => {
                    let total_count: u64 = hist.data_points().map(|dp| dp.count()).sum();
                    assert!(
                        total_count >= 1,
                        "expected snakeway.http.request.duration histogram count >= 1, got {total_count}"
                    );
                }
                other => {
                    panic!(
                        "expected F64 Histogram for snakeway.http.request.duration, got: {other:?}"
                    )
                }
            }
        },
    );
}

/// When no requests are proxied through a running server, no request
/// metrics should be recorded. This verifies metrics are request-driven,
/// not timer-driven.
#[test]
fn no_metrics_recorded_without_requests() {
    // Arrange
    let (metrics, exporter, provider) = setup_metrics();
    let mut cfg = minimal_http_runtime_config();
    let _srv = TestServer::start_with_config_and_metrics(
        &mut cfg,
        integration::harness::upstream::start_http_upstream,
        Some(metrics),
    );

    // Act (no requests sent)

    // Assert
    assert_metric(&exporter, &provider, "snakeway.http.requests", |metric| {
        assert!(
            metric.is_none(),
            "no request counter should exist without traffic"
        );
    });
}

/// The request counter must record attributes for method, status,
/// service, and route. These dimensions are essential for filtering
/// metrics in dashboards and alerting rules.
#[test]
fn request_counter_records_method_and_status_attributes() {
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

    // Assert
    assert_metric(&exporter, &provider, "snakeway.http.requests", |metric| {
        let metric = metric.expect("snakeway.http.requests metric not found");
        match metric.data() {
            AggregatedMetrics::U64(MetricData::Sum(sum)) => {
                let dp = sum
                    .data_points()
                    .next()
                    .expect("expected at least one data point");

                let attrs: std::collections::HashMap<&str, String> = dp
                    .attributes()
                    .map(|kv| (kv.key.as_str(), format!("{}", kv.value)))
                    .collect();

                assert_eq!(
                    attrs.get("method").map(|s| s.as_str()),
                    Some("GET"),
                    "method attribute should be GET; attrs: {attrs:?}"
                );
                assert_eq!(
                    attrs.get("status").map(|s| s.as_str()),
                    Some("2xx"),
                    "status attribute should be 2xx; attrs: {attrs:?}"
                );
                assert!(
                    attrs.contains_key("service"),
                    "service attribute must be present; attrs: {attrs:?}"
                );
                assert!(
                    attrs.contains_key("route"),
                    "route attribute must be present; attrs: {attrs:?}"
                );
            }
            other => panic!("expected U64 Sum for snakeway.http.requests, got: {other:?}"),
        }
    });
}
