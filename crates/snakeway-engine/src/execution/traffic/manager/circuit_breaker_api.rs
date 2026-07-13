use crate::execution::traffic::admin::{
    AdminUpstreamView, CircuitBreakerDetailsView, CircuitBreakerParamsView,
};
use crate::execution::traffic::circuit::CircuitState;
use crate::execution::traffic::{ServiceId, TrafficManager, UpstreamSnapshot};
use crate::runtime::UpstreamId;
use std::sync::atomic::Ordering;

/// Circuit Breaker API
impl TrafficManager {
    /// Called by director when selecting an upstream.
    pub(crate) fn circuit_allows(&self, service_id: &ServiceId, upstream_id: &UpstreamId) -> bool {
        let params = match self.circuit_params.get(service_id) {
            Some(p) => p.clone(),
            None => return true, // fail-open: no config means no circuit
        };

        let key = (service_id.clone(), *upstream_id);
        let mut entry = self.circuit.entry(key).or_default();
        entry.allow_request((service_id, upstream_id), &params)
    }

    /// Called once per request, after we know whether it succeeded.
    /// `started` must be true only if `circuit_allows()` returned true for this request.
    pub(crate) fn circuit_on_end(
        &self,
        service_id: &ServiceId,
        upstream_id: &UpstreamId,
        started: bool,
        success: bool,
    ) {
        let params = match self.circuit_params.get(service_id) {
            Some(p) => p.clone(),
            None => return,
        };

        let key = (service_id.clone(), *upstream_id);
        let mut entry = self.circuit.entry(key).or_default();
        entry.on_request_end((service_id, upstream_id), &params, started, success);
    }

    pub(crate) fn total_requests(&self, service_id: &ServiceId, upstream_id: &UpstreamId) -> u64 {
        self.total_requests
            .get(&(service_id.clone(), *upstream_id))
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    pub(crate) fn total_successes(&self, service_id: &ServiceId, upstream_id: &UpstreamId) -> u64 {
        self.total_successes
            .get(&(service_id.clone(), *upstream_id))
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    pub(crate) fn total_failures(&self, service_id: &ServiceId, upstream_id: &UpstreamId) -> u64 {
        self.total_failures
            .get(&(service_id.clone(), *upstream_id))
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    pub(crate) fn get_upstream_view(
        &self,
        service_id: &ServiceId,
        upstream: &UpstreamSnapshot,
        endpoint_label: &str,
        include_details: bool,
    ) -> AdminUpstreamView {
        let weight = upstream.weight;
        let latency_ms = upstream.latency.as_ref().map(|l| l.ewma.as_millis() as u64);
        let upstream_id = &upstream.endpoint.id();
        let health = self.health_status(service_id, upstream_id);
        let active_requests = self.active_requests(service_id, upstream_id);

        let (total_requests, total_successes, total_failures) = if include_details {
            (
                self.total_requests(service_id, upstream_id),
                self.total_successes(service_id, upstream_id),
                self.total_failures(service_id, upstream_id),
            )
        } else {
            (0, 0, 0)
        };

        let circuit_params = if include_details {
            self.circuit_params
                .get(service_id)
                .map(|p| CircuitBreakerParamsView::from(&**p))
        } else {
            None
        };

        let (circuit_state, circuit_details) = self
            .circuit
            .get(&(service_id.clone(), *upstream_id))
            .map(|c| {
                let details = if include_details {
                    Some(CircuitBreakerDetailsView {
                        consecutive_failures: c.consecutive_failures,
                        opened_at_rfc3339: c
                            .opened_at_system
                            .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()),
                        half_open_in_flight: c.half_open_in_flight,
                        half_open_successes: c.half_open_successes,
                    })
                } else {
                    None
                };
                (c.state(), details)
            })
            .unwrap_or((CircuitState::Closed, None));

        AdminUpstreamView {
            endpoint: endpoint_label.to_owned(),
            weight,
            latency_ms,
            health,
            circuit: circuit_state,
            active_requests,
            total_requests,
            total_successes,
            total_failures,
            circuit_params,
            circuit_details,
        }
    }
}
