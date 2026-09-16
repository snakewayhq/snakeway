use crate::types::runtime::service::upstream_config::UpstreamTcpConfig;
use crate::types::{
    CircuitBreakerConfig, HealthCheckConfig, LoadBalancingStrategy, ServiceSpec, UpstreamUnixConfig,
};
use confval::prelude::{Lower, Report, narrow};
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
        report: &mut Report,
    ) -> Option<Self> {
        let strategy =
            narrow::keyword::<LoadBalancingStrategy>(&spec.load_balancing_strategy, report)?;

        let circuit_breaker = match &spec.circuit_breaker {
            Some(cb) => CircuitBreakerConfig::lower(&cb.value, report)?,
            None => CircuitBreakerConfig::default(),
        };
        let health_check = match &spec.health_check {
            Some(hc) => HealthCheckConfig::lower(&hc.value, report)?,
            None => HealthCheckConfig::default(),
        };

        Some(Self {
            name: name.to_string(),
            listener: listener.to_string(),
            load_balancing_strategy: strategy,
            tcp_upstreams,
            unix_upstreams,
            circuit_breaker,
            health_check,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_config_new_sets_fields() {
        // Arrange
        let spec = ServiceSpec {
            name: confval::source::Located::detached("my-svc".to_string()),
            load_balancing_strategy: confval::source::Located::detached("round_robin".to_string()),
            routes: vec![],
            upstreams: vec![],
            health_check: None,
            circuit_breaker: None,
        };

        // Act
        let config = ServiceConfig::new(
            "my-svc",
            "my-listener",
            vec![],
            vec![],
            &spec,
            &mut Report::new(),
        )
        .unwrap();

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
    fn load_balancing_strategy_all_variants() {
        assert!(matches!(
            LoadBalancingStrategy::try_from("failover").unwrap(),
            LoadBalancingStrategy::Failover
        ));
        assert!(matches!(
            LoadBalancingStrategy::try_from("round_robin").unwrap(),
            LoadBalancingStrategy::RoundRobin
        ));
        assert!(matches!(
            LoadBalancingStrategy::try_from("request_pressure").unwrap(),
            LoadBalancingStrategy::RequestPressure
        ));
        assert!(matches!(
            LoadBalancingStrategy::try_from("sticky_hash").unwrap(),
            LoadBalancingStrategy::StickyHash
        ));
        assert!(matches!(
            LoadBalancingStrategy::try_from("random").unwrap(),
            LoadBalancingStrategy::Random
        ));
        assert!(LoadBalancingStrategy::try_from("psychic").is_err());
    }
}
