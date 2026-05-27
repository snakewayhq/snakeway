use crate::types::runtime::service::upstream_config::UpstreamTcpConfig;
use crate::types::{
    CircuitBreakerConfig, HealthCheckConfig, LoadBalancingStrategySpec, UpstreamUnixConfig,
};
use o2o::o2o;
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

#[derive(o2o, Debug, Clone, Deserialize, Serialize)]
#[from_owned(LoadBalancingStrategySpec)]
pub enum LoadBalancingStrategy {
    Failover,
    RoundRobin,
    RequestPressure,
    StickyHash,
    Random,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_balancing_strategy_failover() {
        // Arrange / Act / Assert
        assert!(matches!(
            LoadBalancingStrategy::from(LoadBalancingStrategySpec::Failover),
            LoadBalancingStrategy::Failover
        ));
    }

    #[test]
    fn load_balancing_strategy_all_variants() {
        // Arrange / Act / Assert
        assert!(matches!(
            LoadBalancingStrategy::from(LoadBalancingStrategySpec::RoundRobin),
            LoadBalancingStrategy::RoundRobin
        ));
        assert!(matches!(
            LoadBalancingStrategy::from(LoadBalancingStrategySpec::RequestPressure),
            LoadBalancingStrategy::RequestPressure
        ));
        assert!(matches!(
            LoadBalancingStrategy::from(LoadBalancingStrategySpec::StickyHash),
            LoadBalancingStrategy::StickyHash
        ));
        assert!(matches!(
            LoadBalancingStrategy::from(LoadBalancingStrategySpec::Random),
            LoadBalancingStrategy::Random
        ));
    }
}
