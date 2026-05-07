use crate::types::{
    CircuitBreakerSpec, HclOrigin, HealthCheckSpec, ServiceRouteSpec, UpstreamSpec,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct ServiceSpec {
    #[serde(skip)]
    pub origin: HclOrigin,
    #[serde(default)]
    pub load_balancing_strategy: LoadBalancingStrategySpec,
    pub routes: Vec<ServiceRouteSpec>,
    pub upstreams: Vec<UpstreamSpec>,
    pub health_check: Option<HealthCheckSpec>,
    pub circuit_breaker: Option<CircuitBreakerSpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalancingStrategySpec {
    #[default]
    Failover,
    RoundRobin,
    RequestPressure,
    StickyHash,
    Random,
}
