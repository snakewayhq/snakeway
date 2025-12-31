use crate::conf::types::{HealthCheckConfig, LoadBalancingStrategy};
use crate::server::UpstreamId;
use crate::server::UpstreamRuntime;
use crate::traffic::snapshot::{ServiceSnapshot, TrafficSnapshot, UpstreamSnapshot};
use crate::traffic::{ServiceId, TrafficManager};
use std::collections::HashMap;

#[test]
fn test_admin_view_counters() {
    let service_id = ServiceId("test_svc".into());
    let upstream_id = UpstreamId(8080);

    let mut services = HashMap::new();
    services.insert(
        service_id.clone(),
        ServiceSnapshot {
            service_id: service_id.clone(),
            strategy: LoadBalancingStrategy::RoundRobin,
            upstreams: vec![UpstreamSnapshot {
                endpoint: UpstreamRuntime {
                    id: upstream_id,
                    host: "127.0.0.1".into(),
                    port: 8080,
                    use_tls: false,
                    sni: "localhost".into(),
                    weight: 1,
                },
                latency: None,
                weight: 1,
            }],
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
    manager.report_success(&service_id, &upstream_id);
    manager.on_request_end(&service_id, &upstream_id);
    manager.report_failure(&service_id, &upstream_id);
    manager.on_request_end(&service_id, &upstream_id);

    let view = manager.get_upstream_view(&service_id, &upstream_id, true);

    assert_eq!(view.total_requests, 2);
    assert_eq!(view.total_successes, 1);
    assert_eq!(view.total_failures, 1);
    assert_eq!(view.active_requests, 0);
}

#[test]
fn test_admin_view_circuit_details() {
    let service_id = ServiceId("test_svc".into());
    let upstream_id = UpstreamId(8080);

    let mut services = HashMap::new();
    services.insert(
        service_id.clone(),
        ServiceSnapshot {
            service_id: service_id.clone(),
            strategy: LoadBalancingStrategy::RoundRobin,
            upstreams: vec![UpstreamSnapshot {
                endpoint: UpstreamRuntime {
                    id: upstream_id,
                    host: "127.0.0.1".into(),
                    port: 8080,
                    use_tls: false,
                    sni: "localhost".into(),
                    weight: 1,
                },
                latency: None,
                weight: 1,
            }],
            circuit_breaker_cfg: crate::conf::types::CircuitBreakerConfig {
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

    let view = manager.get_upstream_view(&service_id, &upstream_id, true);

    assert_eq!(view.circuit, crate::traffic::circuit::CircuitState::Open);
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
                endpoint: UpstreamRuntime {
                    id: upstream_id,
                    host: "127.0.0.1".into(),
                    port: 8080,
                    use_tls: false,
                    sni: "localhost".into(),
                    weight: 1,
                },
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
    manager.report_success(&service_id, &upstream_id);
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
                endpoint: UpstreamRuntime {
                    id: upstream_id2,
                    host: "127.0.0.1".into(),
                    port: 8081,
                    use_tls: false,
                    sni: "localhost".into(),
                    weight: 1,
                },
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
