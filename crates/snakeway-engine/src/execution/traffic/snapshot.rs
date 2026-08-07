use crate::execution::traffic::types::*;
use crate::runtime::{RuntimeState, UpstreamRuntime};
use snakeway_conf::types::LoadBalancingStrategy;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct UpstreamSnapshot {
    pub endpoint: UpstreamRuntime,
    pub(crate) latency: Option<LatencyStats>,
    pub(crate) weight: u32,
}

#[derive(Debug, Clone)]
pub struct ServiceSnapshot {
    #[allow(dead_code)] // useful for debugger inspection
    pub(crate) service_id: ServiceId,
    pub(crate) strategy: LoadBalancingStrategy,
    pub upstreams: Vec<UpstreamSnapshot>,
    pub(crate) circuit_breaker_cfg: snakeway_conf::types::CircuitBreakerConfig,
    pub(crate) health_check_cfg: snakeway_conf::types::HealthCheckConfig,
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
                    weight: u.weight(),
                })
                .collect::<Vec<_>>();

            services.insert(
                ServiceId(name.as_str().into()),
                ServiceSnapshot {
                    service_id: ServiceId(name.as_str().into()),
                    strategy: svc.strategy,
                    upstreams,
                    circuit_breaker_cfg: svc.circuit_breaker_cfg.clone(),
                    health_check_cfg: svc.health_check_cfg.clone(),
                },
            );
        }

        TrafficSnapshot { services }
    }
}
