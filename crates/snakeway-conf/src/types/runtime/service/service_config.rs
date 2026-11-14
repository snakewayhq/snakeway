use crate::types::runtime::service::upstream_config::UpstreamTcpConfig;
use crate::types::{
    CircuitBreakerConfig, CircuitBreakerSpec, HealthCheckConfig, HealthCheckSpec,
    LoadBalancingStrategy, ServiceSpec, UpstreamUnixConfig,
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

        let default_circuit_breaker = CircuitBreakerSpec::default();
        let default_health_check = HealthCheckSpec::default();

        let circuit_breaker = CircuitBreakerConfig::lower(
            spec.circuit_breaker
                .as_ref()
                .map_or(&default_circuit_breaker, |cb| &cb.value),
            report,
        )?;
        let health_check = HealthCheckConfig::lower(
            spec.health_check
                .as_ref()
                .map_or(&default_health_check, |hc| &hc.value),
            report,
        )?;

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

    fn service_spec_with(
        circuit_breaker: Option<CircuitBreakerSpec>,
        health_check: Option<HealthCheckSpec>,
    ) -> ServiceSpec {
        ServiceSpec {
            name: confval::source::Located::detached("my-svc".to_string()),
            load_balancing_strategy: confval::source::Located::detached("round_robin".to_string()),
            routes: vec![],
            upstreams: vec![],
            health_check: health_check.map(confval::source::Located::detached),
            circuit_breaker: circuit_breaker.map(confval::source::Located::detached),
        }
    }

    fn lower(spec: &ServiceSpec) -> ServiceConfig {
        ServiceConfig::new(
            "my-svc",
            "my-listener",
            vec![],
            vec![],
            spec,
            &mut Report::new(),
        )
        .unwrap()
    }

    #[test]
    fn absent_circuit_breaker_block_uses_specification_defaults() {
        // Arrange
        let spec = service_spec_with(None, None);

        // Act
        let config = lower(&spec);

        // Assert
        assert!(
            config.circuit_breaker.count_http_5xx_as_failure,
            "an upstream 5xx must count as a failure, as the specification default declares"
        );
        assert_eq!(config.circuit_breaker.failure_threshold, 5);
        assert_eq!(config.circuit_breaker.open_duration_milliseconds, 10_000);
        assert_eq!(config.circuit_breaker.half_open_max_requests, 1);
        assert_eq!(config.circuit_breaker.success_threshold, 2);
        assert!(!config.circuit_breaker.enable_auto_recovery);
    }

    #[test]
    fn absent_health_check_block_uses_specification_defaults() {
        // Arrange
        let spec = service_spec_with(None, None);

        // Act
        let config = lower(&spec);

        // Assert
        assert!(!config.health_check.enable);
        assert_eq!(config.health_check.failure_threshold, 3);
        assert_eq!(config.health_check.unhealthy_cooldown_seconds, 10);
    }

    #[test]
    fn absent_and_explicit_default_blocks_lower_identically() {
        // Arrange
        let absent = service_spec_with(None, None);
        let explicit = service_spec_with(
            Some(CircuitBreakerSpec::default()),
            Some(HealthCheckSpec::default()),
        );

        // Act
        let (from_absent, from_explicit) = (lower(&absent), lower(&explicit));

        // Assert
        assert_eq!(
            from_absent.circuit_breaker.enable_auto_recovery,
            from_explicit.circuit_breaker.enable_auto_recovery
        );
        assert_eq!(
            from_absent.circuit_breaker.failure_threshold,
            from_explicit.circuit_breaker.failure_threshold
        );
        assert_eq!(
            from_absent.circuit_breaker.open_duration_milliseconds,
            from_explicit.circuit_breaker.open_duration_milliseconds
        );
        assert_eq!(
            from_absent.circuit_breaker.half_open_max_requests,
            from_explicit.circuit_breaker.half_open_max_requests
        );
        assert_eq!(
            from_absent.circuit_breaker.success_threshold,
            from_explicit.circuit_breaker.success_threshold
        );
        assert_eq!(
            from_absent.circuit_breaker.count_http_5xx_as_failure,
            from_explicit.circuit_breaker.count_http_5xx_as_failure
        );
        assert_eq!(
            from_absent.health_check.enable,
            from_explicit.health_check.enable
        );
        assert_eq!(
            from_absent.health_check.failure_threshold,
            from_explicit.health_check.failure_threshold
        );
        assert_eq!(
            from_absent.health_check.unhealthy_cooldown_seconds,
            from_explicit.health_check.unhealthy_cooldown_seconds
        );
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
