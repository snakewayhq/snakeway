use crate::conf::types::LoadBalancingStrategy;
use crate::server::{RuntimeState, UpstreamRuntime};
use crate::traffic::types::*;
use std::collections::HashMap;

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
    pub circuit_breaker_config: crate::conf::types::CircuitBreakerConfig,
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
                .collect::<Vec<_>>();

            services.insert(
                ServiceId(name.clone()),
                ServiceSnapshot {
                    service_id: ServiceId(name.clone()),
                    strategy: svc.strategy.clone(),
                    upstreams,
                    circuit_breaker_config: svc.circuit_breaker.clone(),
                },
            );
        }

        TrafficSnapshot { services }
    }
}
