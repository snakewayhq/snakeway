use crate::conf::types::runtime::service::upstream::UpstreamTcpConfig;
use crate::conf::types::{
    CircuitBreakerConfig, HealthCheckConfig, LoadBalancingStrategySpec, ServiceSpec,
    UpstreamUnixConfig,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceConfig {
    pub name: String,

    /// The name of a listener this service is attached to.
    pub listener: String,

    /// Load balancing strategy
    pub load_balancing_strategy: LoadBalancingStrategy,

    pub tcp_upstreams: Vec<UpstreamTcpConfig>,

    pub unix_upstreams: Vec<UpstreamUnixConfig>,

    pub circuit_breaker: CircuitBreakerConfig,

    pub health_check: HealthCheckConfig,
}

impl ServiceConfig {
    pub fn new(
        name: &str,
        listener: &str,
        tcp_upstreams: Vec<UpstreamTcpConfig>,
        unix_upstreams: Vec<UpstreamUnixConfig>,
        spec: &ServiceSpec,
    ) -> Self {
        Self {
            name: name.to_string(),
            listener: listener.to_string(),
            load_balancing_strategy: spec.load_balancing_strategy.clone().into(),
            tcp_upstreams,
            unix_upstreams,
            circuit_breaker: spec.circuit_breaker.clone().unwrap_or_default(),
            health_check: spec.health_check.clone().unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum LoadBalancingStrategy {
    Failover,
    RoundRobin,
    RequestPressure,
    StickyHash,
    Random,
}

impl From<LoadBalancingStrategySpec> for LoadBalancingStrategy {
    fn from(spec: LoadBalancingStrategySpec) -> Self {
        match spec {
            LoadBalancingStrategySpec::Failover => Self::Failover,
            LoadBalancingStrategySpec::RoundRobin => Self::RoundRobin,
            LoadBalancingStrategySpec::RequestPressure => Self::RequestPressure,
            LoadBalancingStrategySpec::StickyHash => Self::StickyHash,
            LoadBalancingStrategySpec::Random => Self::Random,
        }
    }
}
