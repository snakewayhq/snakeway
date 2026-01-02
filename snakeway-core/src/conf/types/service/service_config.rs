use crate::conf::types::CircuitBreakerConfig;
use crate::conf::types::service::health_check::HealthCheckConfig;
use crate::conf::types::service::upstream::UpstreamConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceConfig {
    pub name: String,

    /// Load balancing strategy
    #[serde(default = "default_strategy")]
    pub strategy: LoadBalancingStrategy,

    #[serde(default)]
    pub upstream: Vec<UpstreamConfig>,

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
