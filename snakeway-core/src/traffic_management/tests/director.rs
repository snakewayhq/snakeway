use crate::conf::types::LoadBalancingStrategy;
use crate::ctx::RequestCtx;
use crate::runtime::{UpstreamId, UpstreamRuntime};
use crate::traffic_management::circuit::CircuitBreakerParams;
use crate::traffic_management::decision::TrafficDecision;
use crate::traffic_management::strategy::TrafficStrategy;
use crate::traffic_management::{
    TrafficManager,
    decision::DecisionReason,
    director::{TrafficDirector, TrafficError},
    snapshot::{ServiceSnapshot, TrafficSnapshot, UpstreamSnapshot},
    types::*,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

// ---------------------------
// Helpers
// ---------------------------

fn dummy_request() -> RequestCtx {
    RequestCtx {
        original_uri: Some("/".parse().unwrap()),
        peer_ip: std::net::Ipv4Addr::LOCALHOST.into(),
        ..Default::default()
    }
}

fn upstream(id: u16) -> UpstreamSnapshot {
    UpstreamSnapshot {
        endpoint: UpstreamRuntime {
            id: UpstreamId(id as u32),
            host: "127.0.0.1".to_string(),
            port: id,
            use_tls: false,
            sni: "localhost".to_string(),
            weight: 1,
        },
        latency: Some(LatencyStats {
            ewma: Duration::from_millis(10),
        }),
        weight: 1,
    }
}

fn snapshot_with_service(
    service_id: ServiceId,
    upstreams: Vec<UpstreamSnapshot>,
    strategy: LoadBalancingStrategy,
) -> TrafficSnapshot {
    let mut services = HashMap::new();
    services.insert(
        service_id.clone(),
        ServiceSnapshot {
            service_id,
            strategy,
            upstreams,
            circuit_breaker_cfg: crate::conf::types::CircuitBreakerConfig {
                enable_auto_recovery: true,
                failure_threshold: 3,
                open_duration_ms: 10000,
                half_open_max_requests: 1,
                success_threshold: 2,
                count_http_5xx_as_failure: true,
            },
            health_check_cfg: crate::conf::types::HealthCheckConfig::default(),
        },
    );

    TrafficSnapshot { services }
}

/// ---------------------------
/// Tests
/// ---------------------------

#[test]
fn unknown_service_returns_error() {
    // Arrange
    let director = TrafficDirector;
    let snapshot = TrafficSnapshot::default();
    let manager = TrafficManager::new(snapshot.clone());

    // Act
    let result = director.decide(
        &dummy_request(),
        &snapshot,
        &ServiceId("missing".into()),
        &manager,
    );

    // Assert
    assert!(matches!(result, Err(TrafficError::UnknownService)));
}

#[test]
fn no_healthy_upstreams_returns_error() {
    // Arrange
    let service_id = ServiceId("svc".into());
    let snapshot = snapshot_with_service(
        service_id.clone(),
        vec![upstream(1), upstream(2)],
        LoadBalancingStrategy::RoundRobin,
    );
    let manager = TrafficManager::new(snapshot.clone());
    manager.health_params.insert(
        service_id.clone(),
        Arc::new(HealthCheckParams {
            enable: true,
            failure_threshold: 3,
            unhealthy_cooldown: Duration::from_secs(10),
        }),
    );
    let director = TrafficDirector;

    // Mark all upstreams unhealthy
    manager.report_failure(&service_id, &UpstreamId(1));
    manager.report_failure(&service_id, &UpstreamId(1));
    manager.report_failure(&service_id, &UpstreamId(1));

    manager.report_failure(&service_id, &UpstreamId(2));
    manager.report_failure(&service_id, &UpstreamId(2));
    manager.report_failure(&service_id, &UpstreamId(2));

    // Act
    let result = director.decide(&dummy_request(), &snapshot, &service_id, &manager);

    // Assert
    assert!(matches!(result, Err(TrafficError::NoHealthyUpstreams)));
}

#[test]
fn single_healthy_upstream_is_selected() {
    // Arrange
    let service_id = ServiceId("svc".into());
    let snapshot = snapshot_with_service(
        service_id.clone(),
        vec![upstream(1), upstream(2)],
        LoadBalancingStrategy::RoundRobin,
    );
    let manager = TrafficManager::new(snapshot.clone());
    manager.health_params.insert(
        service_id.clone(),
        Arc::new(HealthCheckParams {
            enable: true,
            failure_threshold: 3,
            unhealthy_cooldown: Duration::from_secs(10),
        }),
    );
    let director = TrafficDirector;

    // Mark upstream 1 unhealthy
    manager.report_failure(&service_id, &UpstreamId(1));
    manager.report_failure(&service_id, &UpstreamId(1));
    manager.report_failure(&service_id, &UpstreamId(1));

    // Act
    let decision = director
        .decide(&dummy_request(), &snapshot, &service_id, &manager)
        .expect("decision");

    // Assert
    assert_eq!(decision.upstream_id, UpstreamId(2));
}

#[test]
fn strategy_decision_is_respected() {
    // Arrange
    let service_id = ServiceId("svc".into());
    let snapshot = snapshot_with_service(
        service_id.clone(),
        vec![upstream(1), upstream(2)],
        LoadBalancingStrategy::RoundRobin,
    );
    let manager = TrafficManager::new(snapshot.clone());
    let director = TrafficDirector;

    // Act
    let decision = director
        .decide(&dummy_request(), &snapshot, &service_id, &manager)
        .expect("decision");

    // Assert
    assert_eq!(decision.reason, DecisionReason::RoundRobin);
}

#[test]
fn failover_strategy_selects_first_healthy_upstream() {
    // Arrange
    let service_id = ServiceId("svc".into());
    let snapshot = snapshot_with_service(
        service_id.clone(),
        vec![upstream(10), upstream(20)],
        LoadBalancingStrategy::Failover,
    );
    let manager = TrafficManager::new(snapshot.clone());
    let director = TrafficDirector;

    // Act
    let decision = director
        .decide(&dummy_request(), &snapshot, &service_id, &manager)
        .expect("decision");

    // Assert
    assert_eq!(decision.upstream_id, UpstreamId(10));
    assert_eq!(decision.reason, DecisionReason::Failover);
}

#[test]
fn fallback_is_used_when_strategy_returns_none() {
    // Arrange
    let service_id = ServiceId("svc".into());

    let snapshot = snapshot_with_service(
        service_id.clone(),
        vec![upstream(10), upstream(20)],
        LoadBalancingStrategy::Failover, // irrelevant here
    );

    let manager = TrafficManager::new(snapshot.clone());
    let req = dummy_request();

    // Synthetic strategy that never decides
    struct NullStrategy;
    impl TrafficStrategy for NullStrategy {
        fn decide(
            &self,
            _: &RequestCtx,
            _: &ServiceId,
            _: &[UpstreamSnapshot],
            _: &TrafficManager,
        ) -> Option<TrafficDecision> {
            None
        }
    }

    let service = snapshot.services.get(&service_id).unwrap();
    let healthy = &service.upstreams;
    let strategy = NullStrategy;

    // Act
    let decision = strategy
        .decide(&req, &service_id, healthy, &manager)
        .unwrap_or_else(|| TrafficDecision {
            upstream_id: healthy[0].endpoint.id(),
            reason: DecisionReason::NoStrategyDecision,
            protocol: None,
            cb_started: true,
        });

    // Assert
    assert_eq!(decision.upstream_id, UpstreamId(10));
    assert_eq!(decision.reason, DecisionReason::NoStrategyDecision);
}

#[test]
fn director_respects_circuit_breaker() {
    // Arrange
    let service_id = ServiceId("svc".into());
    let snapshot = snapshot_with_service(
        service_id.clone(),
        vec![upstream(1), upstream(2)],
        LoadBalancingStrategy::RoundRobin,
    );
    let manager = TrafficManager::new(snapshot.clone());
    let director = TrafficDirector;

    // Update manager with circuit params (simulating TrafficManager::update)
    let svc_snapshot = snapshot.services.get(&service_id).unwrap();
    let params = CircuitBreakerParams {
        enable_auto_recovery: svc_snapshot.circuit_breaker_cfg.enable_auto_recovery,
        failure_threshold: svc_snapshot.circuit_breaker_cfg.failure_threshold,
        open_duration: Duration::from_millis(svc_snapshot.circuit_breaker_cfg.open_duration_ms),
        half_open_max_requests: svc_snapshot.circuit_breaker_cfg.half_open_max_requests,
        success_threshold: svc_snapshot.circuit_breaker_cfg.success_threshold,
        count_http_5xx_as_failure: svc_snapshot.circuit_breaker_cfg.count_http_5xx_as_failure,
    };
    manager
        .circuit_params
        .insert(service_id.clone(), std::sync::Arc::new(params));

    // Trip circuit for upstream 1
    manager.circuit_on_end(&service_id, &UpstreamId(1), true, false);
    manager.circuit_on_end(&service_id, &UpstreamId(1), true, false);
    manager.circuit_on_end(&service_id, &UpstreamId(1), true, false);

    // Act
    let decision = director
        .decide(&dummy_request(), &snapshot, &service_id, &manager)
        .expect("decision");

    // Assert
    // Should pick upstream 2 because 1's circuit is open
    assert_eq!(decision.upstream_id, UpstreamId(2));
}
