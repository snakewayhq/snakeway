use crate::types::runtime::service::upstream_config::UpstreamTcpConfig;
use crate::types::{CircuitBreakerConfig, HealthCheckConfig, UpstreamUnixConfig};
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum LoadBalancingStrategy {
    Failover,
    RoundRobin,
    RequestPressure,
    StickyHash,
    Random,
}
