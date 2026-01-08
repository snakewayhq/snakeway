use crate::conf::types::service::health_check::HealthCheckConfig;
use crate::conf::types::service::upstream::UpstreamTcpConfig;
use crate::conf::types::{CircuitBreakerConfig, UpstreamUnixConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceConfig {
    pub name: String,

    /// The name of a listener this service is attached to.
    pub listener: String,

    /// Load balancing strategy
    #[serde(default = "default_strategy")]
    pub strategy: LoadBalancingStrategy,

    #[serde(default, rename = "tcp_upstream")]
    pub tcp_upstreams: Vec<UpstreamTcpConfig>,

    #[serde(default, rename = "unix_upstream")]
    pub unix_upstreams: Vec<UpstreamUnixConfig>,

    #[serde(default)]
    pub circuit_breaker: CircuitBreakerConfig,

    #[serde(default)]
    pub health_check: HealthCheckConfig,
}

fn default_strategy() -> LoadBalancingStrategy {
    LoadBalancingStrategy::Failover
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalancingStrategy {
    Failover,
    RoundRobin,
    RequestPressure,
    StickyHash,
    Random,
}
