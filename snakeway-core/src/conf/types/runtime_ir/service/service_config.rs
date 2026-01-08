use crate::conf::types::runtime_ir::service::health_check::HealthCheckConfig;
use crate::conf::types::runtime_ir::service::upstream::UpstreamTcpConfig;
use crate::conf::types::{CircuitBreakerConfig, UpstreamUnixConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceConfig {
    pub name: String,

    /// The name of a listener this service is attached to.
    pub listener: String,

    /// Load balancing strategy
    #[serde(default)]
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

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalancingStrategy {
    #[default]
    Failover,
    RoundRobin,
    RequestPressure,
    StickyHash,
    Random,
}
