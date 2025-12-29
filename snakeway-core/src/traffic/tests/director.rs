use crate::conf::types::LoadBalancingStrategy;
use crate::ctx::RequestCtx;
use crate::server::{UpstreamId, UpstreamRuntime};
use crate::traffic::{
    TrafficManager,
    decision::DecisionReason,
    director::{TrafficDirector, TrafficError},
    snapshot::{ServiceSnapshot, TrafficSnapshot, UpstreamSnapshot},
    types::*,
};
use std::collections::HashMap;
use std::time::Duration;

/// Helper: minimal RequestCtx suitable for routing tests
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

/// Helper: build a healthy upstream snapshot
fn healthy_upstream(id: u16) -> UpstreamSnapshot {
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
        health: HealthStatus { healthy: true },
    }
}

/// Helper: build an unhealthy upstream snapshot
fn unhealthy_upstream(id: u16) -> UpstreamSnapshot {
    let mut u = healthy_upstream(id);
    u.health.healthy = false;
    u
}

/// Helper: snapshot with a single service
fn snapshot_with_service(
    service_id: ServiceId,
    upstreams: Vec<UpstreamSnapshot>,
    strategy: LoadBalancingStrategy,
) -> TrafficSnapshot {
    let service = ServiceSnapshot {
        service_id: service_id.clone(),
        strategy,
        upstreams,
    };

    let mut services = HashMap::new();
    services.insert(service_id, service);

    TrafficSnapshot { services }
}

#[test]
fn unknown_service_returns_error() {
    let director = TrafficDirector;
    let snapshot = TrafficSnapshot {
        services: HashMap::new(),
    };
    let manager = TrafficManager::default();

    let result = director.decide(
        &dummy_request(),
        &snapshot,
        &ServiceId("missing".into()),
        &manager,
    );

    assert!(matches!(result, Err(TrafficError::UnknownService)));
}

#[test]
fn no_healthy_upstreams_returns_error() {
    let director = TrafficDirector;
    let snapshot = snapshot_with_service(
        ServiceId("svc".into()),
        vec![unhealthy_upstream(1), unhealthy_upstream(2)],
        LoadBalancingStrategy::RoundRobin,
    );
    let manager = TrafficManager::default();

    let result = director.decide(
        &dummy_request(),
        &snapshot,
        &ServiceId("svc".into()),
        &manager,
    );

    assert!(matches!(result, Err(TrafficError::NoHealthyUpstreams)));
}

#[test]
fn single_healthy_upstream_is_selected() {
    let director = TrafficDirector;

    let snapshot = snapshot_with_service(
        ServiceId("svc".into()),
        vec![unhealthy_upstream(1), healthy_upstream(2)],
        LoadBalancingStrategy::RoundRobin,
    );
    let manager = TrafficManager::default();

    let decision = director
        .decide(
            &dummy_request(),
            &snapshot,
            &ServiceId("svc".into()),
            &manager,
        )
        .expect("decision");

    assert_eq!(decision.upstream_id, UpstreamId(2));
}

#[test]
fn strategy_decision_is_respected() {
    let director = TrafficDirector;

    let snapshot = snapshot_with_service(
        ServiceId("svc".into()),
        vec![healthy_upstream(1), healthy_upstream(2)],
        LoadBalancingStrategy::RoundRobin,
    );
    let manager = TrafficManager::default();

    let decision = director
        .decide(
            &dummy_request(),
            &snapshot,
            &ServiceId("svc".into()),
            &manager,
        )
        .expect("decision");

    assert_eq!(decision.reason, DecisionReason::RoundRobin);
}

#[test]
fn fallback_used_when_strategy_returns_none() {
    let director = TrafficDirector;
    let snapshot = snapshot_with_service(
        ServiceId("svc".into()),
        vec![healthy_upstream(10), healthy_upstream(20)],
        LoadBalancingStrategy::Failover,
    );
    let manager = TrafficManager::default();

    let decision = director
        .decide(
            &dummy_request(),
            &snapshot,
            &ServiceId("svc".into()),
            &manager,
        )
        .expect("decision");

    assert_eq!(decision.upstream_id, UpstreamId(10));
    assert_eq!(decision.reason, DecisionReason::NoStrategyDecision);
}
