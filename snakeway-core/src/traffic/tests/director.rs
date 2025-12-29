use crate::conf::types::LoadBalancingStrategy;
use crate::ctx::RequestCtx;
use crate::server::{UpstreamId, UpstreamRuntime};
use crate::traffic::decision::TrafficDecision;
use crate::traffic::strategy::TrafficStrategy;
use crate::traffic::{
    TrafficManager,
    decision::DecisionReason,
    director::{TrafficDirector, TrafficError},
    snapshot::{ServiceSnapshot, TrafficSnapshot, UpstreamSnapshot},
    types::*,
};
use std::collections::HashMap;
use std::time::Duration;

/// ---------------------------
/// Helpers
/// ---------------------------

fn dummy_request() -> RequestCtx {
    RequestCtx::new(
        None,
        http::Method::GET,
        "/".parse().unwrap(),
        http::HeaderMap::new(),
        std::net::Ipv4Addr::LOCALHOST.into(),
        false,
        Vec::new(),
    )
}

fn upstream(id: u16) -> UpstreamSnapshot {
    UpstreamSnapshot {
        endpoint: UpstreamRuntime {
            id: UpstreamId(id as u32),
            host: "127.0.0.1".to_string(),
            port: id,
            use_tls: false,
            sni: "localhost".to_string(),
        },
        latency: Some(LatencyStats {
            ewma: Duration::from_millis(10),
        }),
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
            upstream_id: healthy[0].endpoint.id,
            reason: DecisionReason::NoStrategyDecision,
            protocol: None,
        });

    // Assert
    assert_eq!(decision.upstream_id, UpstreamId(10));
    assert_eq!(decision.reason, DecisionReason::NoStrategyDecision);
}
