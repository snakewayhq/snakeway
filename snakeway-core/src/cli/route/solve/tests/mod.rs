use super::*;
use crate::cli::route::solve::solver::solve;
use crate::cli::route::solve::types::{RouteSolveOptions, SyntheticRequest};
use crate::conf::types::LoadBalancingStrategy;
use crate::device::core::registry::DeviceRegistry;
use crate::route::types::RouteId;
use crate::route::{RouteRuntime, Router};
use crate::runtime::{
    RuntimeState, ServiceRuntime, UpstreamId, UpstreamRuntime, UpstreamTcpRuntime,
};
use http::HeaderMap;
use std::collections::HashMap;
use std::sync::Arc;

fn make_state_with_service_route(
    path: &str,
    service_name: &str,
    upstreams: Vec<(&str, u16)>,
) -> RuntimeState {
    let mut router = Router::new();
    router
        .add_route(
            vec!["*".to_string()],
            path,
            RouteRuntime::Service {
                id: RouteId::service(path, service_name),
                upstream: service_name.to_string(),
                allow_websocket: false,
                ws_max_connections: None,
            },
        )
        .unwrap();

    let upstream_rts: Vec<UpstreamRuntime> = upstreams
        .iter()
        .enumerate()
        .map(|(i, (host, port))| {
            UpstreamRuntime::Tcp(UpstreamTcpRuntime {
                id: UpstreamId(i as u32),
                host: host.to_string(),
                port: *port,
                use_tls: false,
                sni: host.to_string(),
                weight: 1,
                verify: false,
                ca: None,
                group_key: 0,
            })
        })
        .collect();

    let mut services = HashMap::new();
    services.insert(
        service_name.to_string(),
        ServiceRuntime {
            strategy: LoadBalancingStrategy::RoundRobin,
            upstreams: upstream_rts,
            circuit_breaker_cfg: Default::default(),
            health_check_cfg: Default::default(),
            listener: Some(Arc::from("listener-0")),
        },
    );

    let mut routers = HashMap::new();
    routers.insert(Arc::from("listener-0") as Arc<str>, router);

    RuntimeState {
        tls: None,
        routers,
        devices: DeviceRegistry::new(),
        services,
    }
}

fn make_req(path: &str) -> SyntheticRequest {
    SyntheticRequest {
        scheme: "http".into(),
        host: "example.com".into(),
        method: http::Method::GET,
        path: path.to_string(),
        query: None,
        headers: HeaderMap::new(),
        client_ip: None,
        body_size: 0,
    }
}

fn opts_default() -> RouteSolveOptions {
    RouteSolveOptions {
        lb_key: None,
        lb_index: None,
        trace: false,
        verbose: false,
    }
}

#[test]
fn solve_lb_key_deterministic() {
    let state = make_state_with_service_route(
        "/api",
        "api-svc",
        vec![("10.0.0.1", 8080), ("10.0.0.2", 8080), ("10.0.0.3", 8080)],
    );
    let req = make_req("/api/foo");

    let opts = RouteSolveOptions {
        lb_key: Some("user-42".into()),
        lb_index: None,
        trace: false,
        verbose: false,
    };

    let d1 = solve(&state, &req, &opts);
    let d2 = solve(&state, &req, &opts);

    assert_eq!(
        d1.selected_upstream, d2.selected_upstream,
        "same lb_key must produce same upstream"
    );
    assert!(d1.selected_upstream.is_some());
}

#[test]
fn solve_lb_index_overrides_lb_key() {
    let state = make_state_with_service_route(
        "/api",
        "api-svc",
        vec![("10.0.0.1", 8080), ("10.0.0.2", 8080)],
    );
    let req = make_req("/api");

    let opts = RouteSolveOptions {
        lb_key: Some("some-key".into()),
        lb_index: Some(1),
        trace: false,
        verbose: false,
    };

    let d = solve(&state, &req, &opts);
    assert_eq!(d.selected_upstream.as_deref(), Some("10.0.0.2:8080"));
}

#[test]
fn solve_default_selects_index_0() {
    let state = make_state_with_service_route(
        "/api",
        "api-svc",
        vec![("10.0.0.1", 8080), ("10.0.0.2", 8080)],
    );
    let req = make_req("/api");
    let d = solve(&state, &req, &opts_default());

    assert_eq!(d.selected_upstream.as_deref(), Some("10.0.0.1:8080"));
}

#[test]
fn solve_no_match() {
    let state = make_state_with_service_route("/api", "api-svc", vec![("10.0.0.1", 8080)]);
    let req = make_req("/other");
    let d = solve(&state, &req, &opts_default());

    assert!(d.matched_route.is_none());
    assert!(d.rejection.is_some());
    assert_eq!(d.rejection.as_ref().unwrap().stage, "route_match");
}

#[test]
fn solve_longest_prefix() {
    let mut router = Router::new();
    router
        .add_route(
            vec!["*".to_string()],
            "/api",
            RouteRuntime::Service {
                id: RouteId::service("/api", "generic-svc"),
                upstream: "generic-svc".to_string(),
                allow_websocket: false,
                ws_max_connections: None,
            },
        )
        .unwrap();
    router
        .add_route(
            vec!["*".to_string()],
            "/api/v2",
            RouteRuntime::Service {
                id: RouteId::service("/api/v2", "v2-svc"),
                upstream: "v2-svc".to_string(),
                allow_websocket: false,
                ws_max_connections: None,
            },
        )
        .unwrap();

    let mut routers = HashMap::new();
    routers.insert(Arc::from("listener-0") as Arc<str>, router);

    let mut services = HashMap::new();
    for name in ["generic-svc", "v2-svc"] {
        services.insert(
            name.to_string(),
            ServiceRuntime {
                strategy: LoadBalancingStrategy::RoundRobin,
                upstreams: vec![UpstreamRuntime::Tcp(UpstreamTcpRuntime {
                    id: UpstreamId(0),
                    host: "127.0.0.1".into(),
                    port: 9000,
                    use_tls: false,
                    sni: "127.0.0.1".into(),
                    weight: 1,
                    verify: false,
                    ca: None,
                    group_key: 0,
                })],
                circuit_breaker_cfg: Default::default(),
                health_check_cfg: Default::default(),
                listener: Some(Arc::from("listener-0")),
            },
        );
    }

    let state = RuntimeState {
        tls: None,
        routers,
        devices: DeviceRegistry::new(),
        services,
    };

    let d = solve(&state, &make_req("/api/v2/users"), &opts_default());
    assert_eq!(d.upstream_service.as_deref(), Some("v2-svc"));

    let d2 = solve(&state, &make_req("/api/v1/users"), &opts_default());
    assert_eq!(d2.upstream_service.as_deref(), Some("generic-svc"));
}

#[test]
fn solve_trace_stable() {
    let state = make_state_with_service_route("/api", "api-svc", vec![("10.0.0.1", 8080)]);
    let req = make_req("/api/test");

    let opts = RouteSolveOptions {
        lb_key: None,
        lb_index: None,
        trace: true,
        verbose: false,
    };

    let d1 = solve(&state, &req, &opts);
    let d2 = solve(&state, &req, &opts);

    let t1 = d1.trace.unwrap();
    let t2 = d2.trace.unwrap();

    assert_eq!(t1.len(), t2.len());
    for (a, b) in t1.iter().zip(t2.iter()) {
        assert_eq!(a.stage, b.stage);
        assert_eq!(a.outcome, b.outcome);
        assert_eq!(a.detail, b.detail);
    }
}

#[test]
fn solve_rejection_stable() {
    let state = make_state_with_service_route("/api", "api-svc", vec![("10.0.0.1", 8080)]);
    let req = make_req("/nope");

    let d1 = solve(&state, &req, &opts_default());
    let d2 = solve(&state, &req, &opts_default());

    let r1 = d1.rejection.unwrap();
    let r2 = d2.rejection.unwrap();
    assert_eq!(r1.stage, r2.stage);
    assert_eq!(r1.reason, r2.reason);
}

#[test]
fn fnv1a_deterministic() {
    let a = fnv1a_hash(b"hello");
    let b = fnv1a_hash(b"hello");
    assert_eq!(a, b);
    assert_ne!(fnv1a_hash(b"hello"), fnv1a_hash(b"world"));
}

#[test]
fn solve_lb_index_wraps() {
    let state = make_state_with_service_route(
        "/api",
        "api-svc",
        vec![("10.0.0.1", 8080), ("10.0.0.2", 8080)],
    );
    let req = make_req("/api");

    let opts = RouteSolveOptions {
        lb_key: None,
        lb_index: Some(5), // 5 % 2 = 1
        trace: false,
        verbose: false,
    };

    let d = solve(&state, &req, &opts);
    assert_eq!(d.selected_upstream.as_deref(), Some("10.0.0.2:8080"));
}

#[test]
fn solve_normalized_populated() {
    let state = make_state_with_service_route("/api", "api-svc", vec![("10.0.0.1", 8080)]);
    let req = SyntheticRequest {
        scheme: "https".into(),
        host: "myhost.com".into(),
        method: http::Method::POST,
        path: "/api/data".into(),
        query: Some("x=1".into()),
        headers: HeaderMap::new(),
        client_ip: Some("192.168.1.1".parse().unwrap()),
        body_size: 1024,
    };

    let d = solve(&state, &req, &opts_default());
    assert_eq!(d.normalized.scheme, "https");
    assert_eq!(d.normalized.host, "myhost.com");
    assert_eq!(d.normalized.method, "POST");
    assert_eq!(d.normalized.path, "/api/data");
    assert_eq!(d.normalized.query.as_deref(), Some("x=1"));
    assert_eq!(d.normalized.client_ip.as_deref(), Some("192.168.1.1"));
    assert_eq!(d.normalized.body_size, 1024);
}
