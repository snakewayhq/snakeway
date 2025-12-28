use crate::conf::types::Strategy;
use crate::ctx::RequestCtx;
use crate::traffic::{
    decision::{DecisionReason, TrafficDecision},
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
fn healthy_upstream(id: u32) -> UpstreamSnapshot {
    UpstreamSnapshot {
        endpoint: UpstreamEndpoint {
            id: UpstreamId(id),
            address: format!("127.0.0.1:{id}"),
            use_tls: false,
        },
        latency: Some(LatencyStats {
            ewma: Duration::from_millis(10),
        }),
        connections: ConnectionStats { active: 0 },
        health: HealthStatus { healthy: true },
    }
}

/// Helper: build an unhealthy upstream snapshot
fn unhealthy_upstream(id: u32) -> UpstreamSnapshot {
    let mut u = healthy_upstream(id);
    u.health.healthy = false;
    u
}

/// Helper: snapshot with a single service
fn snapshot_with_service(
    service_id: ServiceId,
    upstreams: Vec<UpstreamSnapshot>,
    strategy: Strategy,
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
    let director = TrafficDirector::default();
    let snapshot = TrafficSnapshot {
        services: HashMap::new(),
    };

    let result = director.decide(&dummy_request(), &snapshot, &ServiceId("missing".into()));

    assert!(matches!(result, Err(TrafficError::UnknownService)));
}

#[test]
fn no_healthy_upstreams_returns_error() {
    let director = TrafficDirector::default();

    let snapshot = snapshot_with_service(
        ServiceId("svc".into()),
        vec![unhealthy_upstream(1), unhealthy_upstream(2)],
    );

    let result = director.decide(&dummy_request(), &snapshot, &ServiceId("svc".into()));

    assert!(matches!(result, Err(TrafficError::NoHealthyUpstreams)));
}

#[test]
fn single_healthy_upstream_is_selected() {
    let director = TrafficDirector::default();

    let snapshot = snapshot_with_service(
        ServiceId("svc".into()),
        vec![unhealthy_upstream(1), healthy_upstream(2)],
    );

    let decision = director
        .decide(&dummy_request(), &snapshot, &ServiceId("svc".into()))
        .expect("decision");

    assert_eq!(decision.upstream_id, UpstreamId(2));
}

#[test]
fn strategy_decision_is_respected() {
    let director = TrafficDirector::default();

    let snapshot = snapshot_with_service(
        ServiceId("svc".into()),
        vec![healthy_upstream(1), healthy_upstream(2)],
    );

    let decision = director
        .decide(&dummy_request(), &snapshot, &ServiceId("svc".into()))
        .expect("decision");

    assert_eq!(decision.reason, DecisionReason::RoundRobin);
}

#[test]
fn fallback_used_when_strategy_returns_none() {
    struct NullStrategy;

    impl crate::traffic::strategy::TrafficStrategy for NullStrategy {
        fn decide(
            &self,
            _req: &RequestCtx,
            _healthy: &[UpstreamSnapshot],
        ) -> Option<TrafficDecision> {
            None
        }
    }

    let director = TrafficDirector::default();

    let snapshot = snapshot_with_service(
        ServiceId("svc".into()),
        vec![healthy_upstream(10), healthy_upstream(20)],
    );

    let decision = director
        .decide(&dummy_request(), &snapshot, &ServiceId("svc".into()))
        .expect("decision");

    assert_eq!(decision.upstream_id, UpstreamId(10));
    assert_eq!(decision.reason, DecisionReason::ForcedSingle);
}
