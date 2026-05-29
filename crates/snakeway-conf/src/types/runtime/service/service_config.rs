use crate::types::runtime::service::upstream_config::UpstreamTcpConfig;
use crate::types::{
    CircuitBreakerConfig, HealthCheckConfig, LoadBalancingStrategySpec, ServiceSpec,
    UpstreamUnixConfig,
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
            circuit_breaker: spec.circuit_breaker.clone().unwrap_or_default().into(),
            health_check: spec.health_check.clone().unwrap_or_default().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_config_new_sets_fields() {
        // Arrange
        let spec = ServiceSpec {
            origin: Default::default(),
            load_balancing_strategy: LoadBalancingStrategySpec::RoundRobin,
            routes: vec![],
            upstreams: vec![],
            health_check: None,
            circuit_breaker: None,
        };

        // Act
        let config = ServiceConfig::new("my-svc", "my-listener", vec![], vec![], &spec);

        // Assert
        assert_eq!(config.name, "my-svc");
        assert_eq!(config.listener, "my-listener");
        assert!(matches!(
            config.load_balancing_strategy,
            LoadBalancingStrategy::RoundRobin
        ));
        assert!(config.tcp_upstreams.is_empty());
        assert!(config.unix_upstreams.is_empty());
    }

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
