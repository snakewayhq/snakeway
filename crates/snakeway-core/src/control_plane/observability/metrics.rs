use opentelemetry::metrics::{Counter, Gauge, Histogram, Meter};

/// Centralized OTel metric instruments for Snakeway.
///
/// Created once at startup and shared across all request-processing hooks
/// via `GatewayCtx`. When OTel is disabled, no `Metrics` instance is created
/// and all recording call sites are skipped.
pub struct Metrics {
    pub(crate) http_requests: Counter<u64>,
    pub(crate) http_request_duration: Histogram<f64>,
    pub(crate) http_errors: Counter<u64>,
    pub(crate) upstream_active_requests: Gauge<u64>,
    pub(crate) upstream_health: Gauge<u64>,
    pub(crate) circuit_breaker_state: Gauge<u64>,
}

impl Metrics {
    pub(crate) fn new(meter: &Meter) -> Self {
        Self {
            http_requests: meter
                .u64_counter("snakeway.http.requests")
                .with_description("Total HTTP requests processed")
                .build(),
            http_request_duration: meter
                .f64_histogram("snakeway.http.request.duration")
                .with_description("HTTP request duration in milliseconds")
                .with_unit("ms")
                .build(),
            http_errors: meter
                .u64_counter("snakeway.http.errors")
                .with_description("HTTP requests resulting in errors")
                .build(),
            upstream_active_requests: meter
                .u64_gauge("snakeway.upstream.active_requests")
                .with_description("Currently in-flight requests per upstream")
                .build(),
            upstream_health: meter
                .u64_gauge("snakeway.upstream.health")
                .with_description("Upstream health status (1=healthy, 0=unhealthy)")
                .build(),
            circuit_breaker_state: meter
                .u64_gauge("snakeway.circuit_breaker.state")
                .with_description("Circuit breaker state (0=closed, 1=open, 2=half_open)")
                .build(),
        }
    }
}
