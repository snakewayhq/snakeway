use crate::types::runtime::service::upstream_config::UpstreamTcpConfig;
use crate::types::{CircuitBreakerConfig, HealthCheckConfig, ServiceSpec, UpstreamUnixConfig};
use confval::prelude::{Lower, Report};
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

impl TryFrom<&str> for LoadBalancingStrategy {
    type Error = String;

    fn try_from(keyword: &str) -> Result<Self, Self::Error> {
        match keyword {
            "failover" => Ok(LoadBalancingStrategy::Failover),
            "round_robin" => Ok(LoadBalancingStrategy::RoundRobin),
            "request_pressure" => Ok(LoadBalancingStrategy::RequestPressure),
            "sticky_hash" => Ok(LoadBalancingStrategy::StickyHash),
            "random" => Ok(LoadBalancingStrategy::Random),
            other => Err(format!("unknown load_balancing_strategy: {other}")),
        }
    }
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
        let strategy: Result<LoadBalancingStrategy, String> =
            spec.load_balancing_strategy.value.as_str().try_into();
        let strategy = match strategy {
            Ok(strategy) => strategy,
            Err(message) => {
                report
                    .error(message)
                    .at(spec.load_balancing_strategy.span)
                    .emit();
                return None;
            }
        };

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
