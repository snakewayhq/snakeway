use crate::execution::traffic::HealthStatus;
use crate::execution::traffic::circuit::{CircuitBreakerParams, CircuitState};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct AdminUpstreamView {
    pub(crate) endpoint: String,
    pub(crate) weight: u32,
    pub(crate) latency_ms: Option<u64>,
    pub(crate) health: HealthStatus,
    pub(crate) circuit: CircuitState,
    pub(crate) active_requests: u64,
    pub(crate) total_requests: u64,
    pub(crate) total_successes: u64,
    pub(crate) total_failures: u64,
    pub(crate) circuit_params: Option<CircuitBreakerParamsView>,
    pub(crate) circuit_details: Option<CircuitBreakerDetailsView>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CircuitBreakerParamsView {
    pub(crate) enabled: bool,
    pub(crate) failure_threshold: u32,
    pub(crate) open_duration_milliseconds: u64,
    pub(crate) half_open_max_requests: u32,
    pub(crate) success_threshold: u32,
    pub(crate) count_http_5xx_as_failure: bool,
}

impl From<&CircuitBreakerParams> for CircuitBreakerParamsView {
    fn from(p: &CircuitBreakerParams) -> Self {
        Self {
            enabled: p.enable_auto_recovery,
            failure_threshold: p.failure_threshold,
            open_duration_milliseconds: p.open_duration.as_millis() as u64,
            half_open_max_requests: p.half_open_max_requests,
            success_threshold: p.success_threshold,
            count_http_5xx_as_failure: p.count_http_5xx_as_failure,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct CircuitBreakerDetailsView {
    pub(crate) consecutive_failures: u32,
    pub(crate) opened_at_rfc3339: Option<String>,
    pub(crate) half_open_in_flight: u32,
    pub(crate) half_open_successes: u32,
}

#[cfg(test)]
mod tests {
    use crate::execution::traffic::snapshot::{ServiceSnapshot, TrafficSnapshot, UpstreamSnapshot};
    use crate::execution::traffic::{ServiceId, TrafficManager};
    use crate::runtime::{ResolvedAddr, UpstreamId, UpstreamRuntime, UpstreamTcpRuntime};
    use snakeway_conf::types::{HealthCheckConfig, LoadBalancingStrategy};
    use std::collections::HashMap;
    use std::time::Duration;

    #[test]
    fn test_admin_view_counters() {
        let service_id = ServiceId("test_svc".into());
        let upstream_port: u16 = 8080;
        let upstream_id = UpstreamId(upstream_port.into());
        let upstream_host = "127.0.0.1".to_string();
        let upstream_label = format!("{}:{}", upstream_host, upstream_port);
        let upstream_snapshot = UpstreamSnapshot {
            endpoint: UpstreamRuntime::Tcp(UpstreamTcpRuntime {
                id: upstream_id,
                host: upstream_host,
                port: upstream_port,
                resolved_addr: ResolvedAddr::new(([127, 0, 0, 1], upstream_port).into()),
                use_tls: false,
                sni: "localhost".into(),
                weight: 1,
                verify: false,
                ca: None,
                group_key: 0,
            }),
            latency: None,
            weight: 1,
        };
        let mut services = HashMap::new();
        services.insert(
            service_id.clone(),
            ServiceSnapshot {
                service_id: service_id.clone(),
                strategy: LoadBalancingStrategy::RoundRobin,
                upstreams: vec![upstream_snapshot.clone()],
                circuit_breaker_cfg: Default::default(),
                health_check_cfg: HealthCheckConfig {
                    enable: true,
                    ..Default::default()
                },
            },
        );

        let snapshot = TrafficSnapshot { services };
        let manager = TrafficManager::new(snapshot);

        // Simulate some traffic
        manager.on_request_start(&service_id, &upstream_id);
        manager.on_request_start(&service_id, &upstream_id);
        manager.report_success(&service_id, &upstream_id, Duration::from_millis(100));
        manager.on_request_end(&service_id, &upstream_id);
        manager.report_failure(&service_id, &upstream_id);
        manager.on_request_end(&service_id, &upstream_id);

        let view =
            manager.get_upstream_view(&service_id, &upstream_snapshot, &upstream_label, true);

        assert_eq!(view.total_requests, 2);
        assert_eq!(view.total_successes, 1);
        assert_eq!(view.total_failures, 1);
        assert_eq!(view.active_requests, 0);
    }

    #[test]
    fn test_admin_view_circuit_details() {
        let service_id = ServiceId("test_svc".into());
        let upstream_port: u16 = 8080;
        let upstream_id = UpstreamId(upstream_port.into());
        let upstream_host = "127.0.0.1".to_string();
        let upstream_label = format!("{}:{}", upstream_host, upstream_port);
        let upstream_snapshot = UpstreamSnapshot {
            endpoint: UpstreamRuntime::Tcp(UpstreamTcpRuntime {
                id: upstream_id,
                host: upstream_host,
                port: upstream_port,
                resolved_addr: ResolvedAddr::new(([127, 0, 0, 1], upstream_port).into()),
                use_tls: false,
                sni: "localhost".into(),
                weight: 1,
                verify: false,
                ca: None,
                group_key: 0,
            }),
            latency: None,
            weight: 1,
        };

        let mut services = HashMap::new();
        services.insert(
            service_id.clone(),
            ServiceSnapshot {
                service_id: service_id.clone(),
                strategy: LoadBalancingStrategy::RoundRobin,
                upstreams: vec![upstream_snapshot.clone()],
                circuit_breaker_cfg: snakeway_conf::types::CircuitBreakerConfig {
                    enable_auto_recovery: true,
                    failure_threshold: 2,
                    ..Default::default()
                },
                health_check_cfg: Default::default(),
            },
        );

        let snapshot = TrafficSnapshot { services };
        let manager = TrafficManager::new(snapshot.clone());
        manager.update(snapshot); // To populate circuit_params

        // Trip the circuit
        manager.circuit_on_end(&service_id, &upstream_id, true, false);
        manager.circuit_on_end(&service_id, &upstream_id, true, false);

        let view =
            manager.get_upstream_view(&service_id, &upstream_snapshot, &upstream_label, true);

        assert_eq!(
            view.circuit,
            crate::execution::traffic::circuit::CircuitState::Open
        );
        let details = view.circuit_details.expect("details");
        assert!(details.opened_at_rfc3339.is_some());

        // Params should be present
        let params = view.circuit_params.expect("params");
        assert!(params.enabled);
        assert_eq!(params.failure_threshold, 2);
    }

    #[test]
    fn test_metrics_persistence_on_reload() {
        let service_id = ServiceId("test_svc".into());
        let upstream_id = UpstreamId(8080);

        let mut services = HashMap::new();
        services.insert(
            service_id.clone(),
            ServiceSnapshot {
                service_id: service_id.clone(),
                strategy: LoadBalancingStrategy::RoundRobin,
                upstreams: vec![UpstreamSnapshot {
                    endpoint: UpstreamRuntime::Tcp(UpstreamTcpRuntime {
                        id: upstream_id,
                        host: "127.0.0.1".into(),
                        port: 8080,
                        resolved_addr: ResolvedAddr::new(([127, 0, 0, 1], 8080).into()),
                        use_tls: false,
                        sni: "localhost".into(),
                        weight: 1,
                        verify: false,
                        ca: None,
                        group_key: 0,
                    }),
                    latency: None,
                    weight: 1,
                }],
                circuit_breaker_cfg: Default::default(),
                health_check_cfg: Default::default(),
            },
        );

        let snapshot = TrafficSnapshot {
            services: services.clone(),
        };
        let manager = TrafficManager::new(snapshot.clone());

        // Record traffic
        manager.on_request_start(&service_id, &upstream_id);
        manager.report_success(&service_id, &upstream_id, Duration::from_millis(100));
        manager.on_request_end(&service_id, &upstream_id);

        assert_eq!(manager.total_requests(&service_id, &upstream_id), 1);

        // Reload with same upstream
        manager.update(snapshot);

        // Counters should persist
        assert_eq!(manager.total_requests(&service_id, &upstream_id), 1);

        // Reload with different upstream
        let mut services2 = HashMap::new();
        let upstream_id2 = UpstreamId(8081);
        services2.insert(
            service_id.clone(),
            ServiceSnapshot {
                service_id: service_id.clone(),
                strategy: LoadBalancingStrategy::RoundRobin,
                upstreams: vec![UpstreamSnapshot {
                    endpoint: UpstreamRuntime::Tcp(UpstreamTcpRuntime {
                        id: upstream_id2,
                        host: "127.0.0.1".into(),
                        port: 8081,
                        resolved_addr: ResolvedAddr::new(([127, 0, 0, 1], 8081).into()),
                        use_tls: false,
                        sni: "localhost".into(),
                        weight: 1,
                        verify: false,
                        ca: None,
                        group_key: 0,
                    }),
                    latency: None,
                    weight: 1,
                }],
                circuit_breaker_cfg: Default::default(),
                health_check_cfg: Default::default(),
            },
        );
        manager.update(TrafficSnapshot {
            services: services2,
        });

        // Old upstream's counters should be cleaned up
        assert_eq!(manager.total_requests(&service_id, &upstream_id), 0);
    }
}
