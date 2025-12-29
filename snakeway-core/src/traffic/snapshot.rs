use crate::conf::types::LoadBalancingStrategy;
use crate::server::{RuntimeState, UpstreamRuntime};
use crate::traffic::circuit::CircuitBreakerParams;
use crate::traffic::types::*;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct UpstreamSnapshot {
    pub endpoint: UpstreamRuntime,
    pub latency: Option<LatencyStats>,
}

#[derive(Debug, Clone)]
pub struct ServiceSnapshot {
    pub service_id: ServiceId,
    pub strategy: LoadBalancingStrategy,
    pub upstreams: Vec<UpstreamSnapshot>,
    pub circuit: CircuitBreakerParams,
}

/// Immutable, control-plane snapshot of traffic topology and health.
///
/// Safe to read from the request hot path.
/// Updated only by reload, health checks, or discovery.
#[derive(Debug, Clone, Default)]
pub struct TrafficSnapshot {
    pub services: HashMap<ServiceId, ServiceSnapshot>,
}

impl TrafficSnapshot {
    pub fn from_runtime(state: &RuntimeState) -> Self {
        let mut services = HashMap::new();

        for (name, svc) in &state.services {
            let upstreams = svc
                .upstreams
                .iter()
                .map(|u| UpstreamSnapshot {
                    endpoint: u.clone(),
                    latency: None,
                })
                .collect();

            services.insert(
                ServiceId(name.clone()),
                ServiceSnapshot {
                    service_id: ServiceId(name.clone()),
                    strategy: svc.strategy.clone(),
                    upstreams,
                    circuit: CircuitBreakerParams {
                        enabled: svc.circuit_breaker.enabled,
                        failure_threshold: svc.circuit_breaker.failure_threshold,
                        open_duration: Duration::from_millis(svc.circuit_breaker.open_duration_ms),
                        half_open_max_requests: svc.circuit_breaker.half_open_max_requests,
                        success_threshold: svc.circuit_breaker.success_threshold,
                        count_http_5xx_as_failure: svc.circuit_breaker.count_http_5xx_as_failure,
                    },
                },
            );
        }

        TrafficSnapshot { services }
    }
}
