use crate::proxy::TrafficProxy;
use opentelemetry::{KeyValue, Value};
use snakeway_engine::ctx::RequestCtx;
use snakeway_engine::traffic::{TransportFailure, UpstreamOutcome};

impl TrafficProxy {
    pub(crate) fn record_metrics(&self, ctx: &RequestCtx) {
        use snakeway_engine::traffic::circuit::CircuitState;

        let Some(metrics) = &self.proxy_ctx.metrics else {
            return;
        };

        let snapshot = self.proxy_ctx.traffic_manager.snapshot();
        let selected_service = ctx
            .selected_upstream
            .as_ref()
            .and_then(|(service_id, _)| snapshot.services.get(service_id));

        let service = ctx.service.as_deref().unwrap_or("unknown");
        let route = ctx
            .route_id
            .as_ref()
            .map(|r| r.as_str())
            .unwrap_or_else(|| "unknown".into());
        let method = ctx.method_str();

        let status = match &ctx.upstream_outcome {
            Some(UpstreamOutcome::Success) => "2xx",
            Some(UpstreamOutcome::HttpStatus(s)) if *s >= 500 => "5xx",
            Some(UpstreamOutcome::HttpStatus(s)) if *s >= 400 => "4xx",
            Some(UpstreamOutcome::HttpStatus(_)) => "other",
            Some(UpstreamOutcome::Transport(_)) => "transport_error",
            None => "no_upstream",
        };

        let service_attr = match selected_service {
            Some(selected) => selected.metric_service().clone(),
            None => KeyValue::new("service", service.to_string()),
        };

        let request_attrs = &[
            KeyValue::new("method", method_value(method)),
            KeyValue::new("status", status),
            service_attr,
            KeyValue::new("route", route),
        ];

        metrics.http_requests.add(1, request_attrs);

        // Duration and upstream-scoped metrics.
        if let Some((service_id, upstream_id)) = &ctx.selected_upstream {
            let duration_ms = ctx.request_start.elapsed().as_secs_f64() * 1000.0;
            let cached = selected_service
                .and_then(|selected| {
                    selected
                        .upstreams
                        .iter()
                        .find(|upstream| upstream.endpoint.id() == *upstream_id)
                })
                .map(|upstream| upstream.metric_attributes());

            // A reload can replace the snapshot between selection and here, so the upstream
            // may be gone from it. Build the attributes for that one request instead.
            let built;
            let upstream_attrs: &[KeyValue] = match cached {
                Some(attributes) => attributes,
                None => {
                    built = [
                        KeyValue::new("service", service_id.as_str().to_string()),
                        KeyValue::new("upstream", upstream_id.as_u32().to_string()),
                    ];
                    &built
                }
            };

            metrics
                .http_request_duration
                .record(duration_ms, upstream_attrs);

            // Error counter.
            match &ctx.upstream_outcome {
                Some(UpstreamOutcome::HttpStatus(s)) if *s >= 500 => {
                    metrics.http_errors.add(
                        1,
                        &[
                            upstream_attrs[0].clone(),
                            upstream_attrs[1].clone(),
                            KeyValue::new("error.type", "http_5xx"),
                        ],
                    );
                }
                Some(UpstreamOutcome::Transport(failure)) => {
                    let error_type = match failure {
                        TransportFailure::Connect => "connect",
                        TransportFailure::Timeout => "timeout",
                        TransportFailure::Reset => "reset",
                        TransportFailure::Protocol => "protocol",
                        TransportFailure::Tls => "tls",
                    };
                    metrics.http_errors.add(
                        1,
                        &[
                            upstream_attrs[0].clone(),
                            upstream_attrs[1].clone(),
                            KeyValue::new("error.type", error_type),
                        ],
                    );
                }
                _ => {}
            }

            // Gauge: active requests.
            let tm = &self.proxy_ctx.traffic_manager;
            metrics
                .upstream_active_requests
                .record(tm.active_requests(service_id, upstream_id), upstream_attrs);

            // Gauge: health status.
            let healthy = tm.health_status(service_id, upstream_id).healthy;
            metrics
                .upstream_health
                .record(u64::from(healthy), upstream_attrs);

            // Gauge: circuit breaker state.
            if let Some(state) = tm.circuit_state(service_id, upstream_id) {
                let state_value = match state {
                    CircuitState::Closed => 0,
                    CircuitState::Open => 1,
                    CircuitState::HalfOpen => 2,
                };
                metrics
                    .circuit_breaker_state
                    .record(state_value, upstream_attrs);
            }
        }
    }
}

/// A standard method maps to a static string, so the common case does not allocate.
fn method_value(method: &str) -> Value {
    const STANDARD: [&str; 9] = [
        "GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "PATCH", "CONNECT", "TRACE",
    ];
    match STANDARD.iter().find(|standard| **standard == method) {
        Some(standard) => Value::from(*standard),
        None => Value::from(method.to_string()),
    }
}
