use opentelemetry::metrics::MeterProvider;
use opentelemetry_sdk::metrics::data::{AggregatedMetrics, Metric, MetricData};
use opentelemetry_sdk::metrics::{InMemoryMetricExporter, PeriodicReader, SdkMeterProvider};
use reqwest::StatusCode;
use snakeway_core::testing_api::observability::Metrics;
use snakeway_tests::conf::minimal_http_runtime_config;
use snakeway_tests::harness::TestServer;
use std::sync::Arc;
use std::time::{Duration, Instant};

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
///
/// Single-shot: this is for assertions about a metric's *absence*, where
/// waiting cannot change the answer.
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

/// Poll until the named metric appears, then apply the assertion to it.
///
/// Request metrics are recorded in Pingora's `logging` hook, which runs
/// *after* the response has been flushed to the downstream client. The client
/// receiving its response is therefore not a synchronization point for the
/// server-side recording: by the time `srv.get(...).send()` returns, the hook
/// may not have run yet. A single `force_flush` races that hook and flakes
/// under load (notably in CI). Retrying the flush over a short window absorbs
/// the gap; once the hook records the metric, the next cumulative flush sees
/// it.
fn assert_metric_present(
    exporter: &InMemoryMetricExporter,
    provider: &SdkMeterProvider,
    name: &str,
    assertion: impl FnOnce(&Metric),
) {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut assertion = Some(assertion);
    loop {
        // Reset first so each poll reads only the latest cumulative export.
        // The exporter accumulates every batch, so without this the search
        // would keep returning the earliest (partial) one.
        exporter.reset();
        provider.force_flush().unwrap();
        let finished = exporter.get_finished_metrics().unwrap();
        if let Some(metric) = finished
            .iter()
            .flat_map(|rm| rm.scope_metrics())
            .flat_map(|sm| sm.metrics())
            .find(|m| m.name() == name)
        {
            (assertion.take().expect("assertion runs once"))(metric);
            return;
        }
        assert!(
            Instant::now() < deadline,
            "{name} metric not found within timeout"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
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
        snakeway_tests::harness::upstream::start_http_upstream,
        Some(metrics),
    );

    // Act
    let res = srv.get("/api").send().unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Assert
    assert_metric_present(
        &exporter,
        &provider,
        "snakeway.http.requests",
        |metric| match metric.data() {
            AggregatedMetrics::U64(MetricData::Sum(sum)) => {
                let total: u64 = sum.data_points().map(|dp| dp.value()).sum();
                assert!(
                    total >= 1,
                    "expected snakeway.http.requests counter >= 1, got {total}"
                );
            }
            other => panic!("expected U64 Sum for snakeway.http.requests, got: {other:?}"),
        },
    );
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
        snakeway_tests::harness::upstream::start_http_upstream,
        Some(metrics),
    );

    // Act
    let res = srv.get("/api").send().unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Assert
    assert_metric_present(
        &exporter,
        &provider,
        "snakeway.http.request.duration",
        |metric| match metric.data() {
            AggregatedMetrics::F64(MetricData::Histogram(hist)) => {
                let total_count: u64 = hist.data_points().map(|dp| dp.count()).sum();
                assert!(
                    total_count >= 1,
                    "expected snakeway.http.request.duration histogram count >= 1, got {total_count}"
                );
            }
            other => {
                panic!("expected F64 Histogram for snakeway.http.request.duration, got: {other:?}")
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
        snakeway_tests::harness::upstream::start_http_upstream,
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
        snakeway_tests::harness::upstream::start_http_upstream,
        Some(metrics),
    );

    // Act
    let res = srv.get("/api").send().unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Assert
    assert_metric_present(
        &exporter,
        &provider,
        "snakeway.http.requests",
        |metric| match metric.data() {
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
        },
    );
}

/// When N concurrent requests are sent, the request counter must record
/// exactly N. This verifies atomic metric recording under concurrent load.
#[test]
fn concurrent_requests_counted_accurately() {
    // Arrange
    let (metrics, exporter, provider) = setup_metrics();
    let mut cfg = minimal_http_runtime_config();
    let srv = TestServer::start_with_config_and_metrics(
        &mut cfg,
        snakeway_tests::harness::upstream::start_http_upstream,
        Some(metrics),
    );
    let n = 20u64;

    // Act: send N concurrent requests.
    let base_url = srv.base_url();
    std::thread::scope(|s| {
        for _ in 0..n {
            let url = base_url.join("/api").unwrap();
            s.spawn(move || {
                let client = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(5))
                    .build()
                    .unwrap();
                let res = client
                    .get(url)
                    .header("Host", snakeway_tests::constants::TEST_HOST)
                    .send()
                    .unwrap();
                assert_eq!(res.status(), StatusCode::OK);
            });
        }
    });

    // Assert: poll until all N logging hooks have recorded. `logging` runs
    // after each response is flushed downstream, so a single flush can catch a
    // partial count; wait for the total to reach N, then assert it is exactly
    // N (which also catches over-counting).
    let deadline = Instant::now() + Duration::from_secs(5);
    let total = loop {
        let total = request_counter_total(&exporter, &provider);
        if total >= n {
            break total;
        }
        assert!(
            Instant::now() < deadline,
            "expected {n} requests counted, last saw {total}"
        );
        std::thread::sleep(Duration::from_millis(25));
    };
    assert_eq!(
        total, n,
        "expected exactly {n} requests counted, got {total}"
    );
}

/// Flush and return the summed value of the `snakeway.http.requests` U64 Sum
/// counter across all data points, or 0 if it has not been recorded yet.
fn request_counter_total(exporter: &InMemoryMetricExporter, provider: &SdkMeterProvider) -> u64 {
    // Reset first so we read only the latest cumulative export; the exporter
    // otherwise accumulates every prior (partial-count) batch.
    exporter.reset();
    provider.force_flush().unwrap();
    let finished = exporter.get_finished_metrics().unwrap();
    finished
        .iter()
        .flat_map(|rm| rm.scope_metrics())
        .flat_map(|sm| sm.metrics())
        .find(|m| m.name() == "snakeway.http.requests")
        .map(|m| match m.data() {
            AggregatedMetrics::U64(MetricData::Sum(sum)) => {
                sum.data_points().map(|dp| dp.value()).sum()
            }
            _ => 0,
        })
        .unwrap_or(0)
}
