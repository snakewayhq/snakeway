use crate::cli::route::solve::types::{
    RouteSolveDecision, RouteSolveNormalized, RouteSolveOptions, RouteSolveRejection,
    RouteSolveTraceStep, SyntheticRequest,
};
use snakeway_engine::execution::route::{RouteEntry, RouteRuntime};
use snakeway_engine::runtime::{RuntimeState, ServiceRuntime, UpstreamRuntime};

/// Deterministic, side-effect-free route resolution.
///
/// Walks the same router matching path the proxy uses, then selects an
/// upstream according to the supplied [`RouteSolveOptions`].
pub fn walk_solve(
    state: &RuntimeState,
    req: &SyntheticRequest,
    opts: &RouteSolveOptions,
) -> RouteSolveDecision {
    let mut trace: Vec<RouteSolveTraceStep> = Vec::new();
    let record_trace = opts.trace || opts.verbose;

    let normalized = RouteSolveNormalized {
        scheme: req.scheme.clone(),
        host: req.host.clone(),
        method: req.method.to_string(),
        path: req.path.clone(),
        query: req.query.clone(),
        client_ip: req.client_ip.map(|ip| ip.to_string()),
        body_size: req.body_size,
    };

    // --- Step 1: find the router for the first listener -----------------------
    // In a dry-run we don't have a listener name from a socket, so we try each
    // router in deterministic (sorted-key) order and pick the first match.
    let mut sorted_listeners: Vec<_> = state.routers.keys().collect();
    sorted_listeners.sort();

    let mut matched_entry: Option<(&RouteEntry, &str)> = None;

    for listener_key in &sorted_listeners {
        let router = &state.routers[*listener_key];
        if record_trace {
            trace.push(RouteSolveTraceStep {
                stage: "route_match".into(),
                outcome: "trying".into(),
                detail: format!(
                    "listener={}, host={}, path={}",
                    listener_key, req.host, req.path
                ),
            });
        }
        match router.match_route(&req.host, &req.path) {
            Ok(entry) => {
                if record_trace {
                    trace.push(RouteSolveTraceStep {
                        stage: "route_match".into(),
                        outcome: "matched".into(),
                        detail: format!(
                            "listener={}, route_path={}, id={}",
                            listener_key,
                            entry.path,
                            entry.kind.id().as_str()
                        ),
                    });
                }
                matched_entry = Some((entry, listener_key));
                break;
            }
            Err(_) => {
                if record_trace {
                    trace.push(RouteSolveTraceStep {
                        stage: "route_match".into(),
                        outcome: "no_match".into(),
                        detail: format!(
                            "listener={}, host={}, path={}",
                            listener_key, req.host, req.path
                        ),
                    });
                }
            }
        }
    }

    let (entry, _listener) = match matched_entry {
        Some(pair) => pair,
        None => {
            if record_trace {
                trace.push(RouteSolveTraceStep {
                    stage: "route_match".into(),
                    outcome: "rejected".into(),
                    detail: format!("no route matched host={} path={}", req.host, req.path),
                });
            }
            return RouteSolveDecision {
                matched_route: None,
                route_kind: None,
                upstream_service: None,
                selected_upstream: None,
                static_file_dir: None,
                rejection: Some(RouteSolveRejection {
                    stage: "route_match".into(),
                    reason: format!("no route matched host={} path={}", req.host, req.path),
                }),
                normalized,
                trace: if record_trace { Some(trace) } else { None },
            };
        }
    };

    // --- Step 2: extract route details ----------------------------------------
    let route_id = entry.kind.id().as_str();
    match &entry.kind {
        RouteRuntime::Service {
            upstream: service_name,
            ..
        } => {
            let svc = state.services.get(service_name.as_str());
            let (selected, svc_trace) = match svc {
                Some(svc_rt) => select_upstream(svc_rt, opts, record_trace),
                None => {
                    let step = if record_trace {
                        Some(RouteSolveTraceStep {
                            stage: "upstream_select".into(),
                            outcome: "rejected".into(),
                            detail: format!("service '{}' not found in runtime", service_name),
                        })
                    } else {
                        None
                    };
                    (None, step.into_iter().collect())
                }
            };
            trace.extend(svc_trace);

            RouteSolveDecision {
                matched_route: Some(route_id),
                route_kind: Some("service".into()),
                upstream_service: Some(service_name.clone()),
                selected_upstream: selected,
                static_file_dir: None,
                rejection: None,
                normalized,
                trace: if record_trace { Some(trace) } else { None },
            }
        }
        RouteRuntime::Static { file_dir, path, .. } => {
            if record_trace {
                trace.push(RouteSolveTraceStep {
                    stage: "static_resolve".into(),
                    outcome: "resolved".into(),
                    detail: format!("path={}, file_dir={}", path, file_dir.display()),
                });
            }
            RouteSolveDecision {
                matched_route: Some(route_id),
                route_kind: Some("static".into()),
                upstream_service: None,
                selected_upstream: None,
                static_file_dir: Some(file_dir.display().to_string()),
                rejection: None,
                normalized,
                trace: if record_trace { Some(trace) } else { None },
            }
        }
    }
}

/// Deterministic upstream selection (no randomness, no clock).
fn select_upstream(
    svc: &ServiceRuntime,
    opts: &RouteSolveOptions,
    record_trace: bool,
) -> (Option<String>, Vec<RouteSolveTraceStep>) {
    let mut trace = Vec::new();
    let upstreams = &svc.upstreams;

    if upstreams.is_empty() {
        if record_trace {
            trace.push(RouteSolveTraceStep {
                stage: "upstream_select".into(),
                outcome: "rejected".into(),
                detail: "service has no upstreams".into(),
            });
        }
        return (None, trace);
    }

    let (idx, rule) = if let Some(forced) = opts.lb_index {
        let idx = forced % upstreams.len();
        (
            idx,
            format!("lb_index={} (mod {})", forced, upstreams.len()),
        )
    } else if let Some(ref key) = opts.lb_key {
        let hash = fnv1a_hash(key.as_bytes());
        let idx = (hash as usize) % upstreams.len();
        (
            idx,
            format!("lb_key=\"{}\" hash={} (mod {})", key, hash, upstreams.len()),
        )
    } else {
        (0, "default index 0".into())
    };

    let selected = &upstreams[idx];
    let authority = upstream_authority(selected);

    if record_trace {
        trace.push(RouteSolveTraceStep {
            stage: "upstream_select".into(),
            outcome: "selected".into(),
            detail: format!("rule={}, idx={}, upstream={}", rule, idx, authority),
        });
    }

    (Some(authority), trace)
}

fn upstream_authority(u: &UpstreamRuntime) -> String {
    u.authority()
}

/// FNV-1a (32-bit) with a fixed implementation - fully deterministic.
pub(crate) fn fnv1a_hash(data: &[u8]) -> u32 {
    const FNV_OFFSET: u32 = 2166136261;
    const FNV_PRIME: u32 = 16777619;
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::route::solve::types::{RouteSolveOptions, SyntheticRequest};
    use snakeway_conf::types::LoadBalancingStrategy;
    use snakeway_engine::execution::device::core::DeviceRegistry;
    use snakeway_engine::execution::route::types::RouteId;
    use snakeway_engine::execution::route::{RouteRuntime, Router};
    use snakeway_engine::runtime::{
        ResolvedAddr, RuntimeState, ServiceRuntime, UpstreamId, UpstreamRuntime, UpstreamTcpRuntime,
    };
    use std::collections::HashMap;
    use std::net::ToSocketAddrs;
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
                    resolved_addr: ResolvedAddr::new(
                        (*host, *port).to_socket_addrs().unwrap().next().unwrap(),
                    ),
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
        // Arrange
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

        // Act
        let (d1, d2) = (
            walk_solve(&state, &req, &opts),
            walk_solve(&state, &req, &opts),
        );

        // Assert
        assert_eq!(
            d1.selected_upstream, d2.selected_upstream,
            "same lb_key must produce same upstream"
        );
        assert!(d1.selected_upstream.is_some());
    }

    #[test]
    fn solve_lb_index_overrides_lb_key() {
        // Arrange
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

        // Act
        let d = walk_solve(&state, &req, &opts);

        // Assert
        assert_eq!(d.selected_upstream.as_deref(), Some("10.0.0.2:8080"));
    }

    #[test]
    fn solve_default_selects_index_0() {
        // Arrange
        let state = make_state_with_service_route(
            "/api",
            "api-svc",
            vec![("10.0.0.1", 8080), ("10.0.0.2", 8080)],
        );
        let req = make_req("/api");

        // Act
        let d = walk_solve(&state, &req, &opts_default());

        // Assert
        assert_eq!(d.selected_upstream.as_deref(), Some("10.0.0.1:8080"));
    }

    #[test]
    fn solve_no_match() {
        // Arrange
        let state = make_state_with_service_route("/api", "api-svc", vec![("10.0.0.1", 8080)]);
        let req = make_req("/other");

        // Act
        let d = walk_solve(&state, &req, &opts_default());

        // Assert
        assert!(d.matched_route.is_none());
        assert!(d.rejection.is_some());
        assert_eq!(d.rejection.as_ref().unwrap().stage, "route_match");
    }

    #[test]
    fn solve_longest_prefix() {
        // Arrange
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
                        resolved_addr: ResolvedAddr::new("127.0.0.1:9000".parse().unwrap()),
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

        // Act
        let (d, d2) = (
            walk_solve(&state, &make_req("/api/v2/users"), &opts_default()),
            walk_solve(&state, &make_req("/api/v1/users"), &opts_default()),
        );

        // Assert
        assert_eq!(d.upstream_service.as_deref(), Some("v2-svc"));
        assert_eq!(d2.upstream_service.as_deref(), Some("generic-svc"));
    }

    #[test]
    fn solve_trace_stable() {
        // Arrange
        let state = make_state_with_service_route("/api", "api-svc", vec![("10.0.0.1", 8080)]);
        let req = make_req("/api/test");
        let opts = RouteSolveOptions {
            lb_key: None,
            lb_index: None,
            trace: true,
            verbose: false,
        };

        // Act
        let (d1, d2) = (
            walk_solve(&state, &req, &opts),
            walk_solve(&state, &req, &opts),
        );

        // Assert
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
        // Arrange
        let state = make_state_with_service_route("/api", "api-svc", vec![("10.0.0.1", 8080)]);
        let req = make_req("/nope");

        // Act
        let (d1, d2) = (
            walk_solve(&state, &req, &opts_default()),
            walk_solve(&state, &req, &opts_default()),
        );

        // Assert
        let r1 = d1.rejection.unwrap();
        let r2 = d2.rejection.unwrap();
        assert_eq!(r1.stage, r2.stage);
        assert_eq!(r1.reason, r2.reason);
    }

    #[test]
    fn fnv1a_deterministic() {
        // Arrange and Act
        let (a, b) = (fnv1a_hash(b"hello"), fnv1a_hash(b"hello"));

        // Assert
        assert_eq!(a, b);
        assert_ne!(fnv1a_hash(b"hello"), fnv1a_hash(b"world"));
    }

    #[test]
    fn solve_lb_index_wraps() {
        // Arrange
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

        // Act
        let d = walk_solve(&state, &req, &opts);

        // Assert
        assert_eq!(d.selected_upstream.as_deref(), Some("10.0.0.2:8080"));
    }

    #[test]
    fn solve_normalized_populated() {
        // Arrange
        let state = make_state_with_service_route("/api", "api-svc", vec![("10.0.0.1", 8080)]);
        let req = SyntheticRequest {
            scheme: "https".into(),
            host: "myhost.com".into(),
            method: http::Method::POST,
            path: "/api/data".into(),
            query: Some("x=1".into()),
            client_ip: Some("192.168.1.1".parse().unwrap()),
            body_size: 1024,
        };

        // Act
        let d = walk_solve(&state, &req, &opts_default());

        // Assert
        assert_eq!(d.normalized.scheme, "https");
        assert_eq!(d.normalized.host, "myhost.com");
        assert_eq!(d.normalized.method, "POST");
        assert_eq!(d.normalized.path, "/api/data");
        assert_eq!(d.normalized.query.as_deref(), Some("x=1"));
        assert_eq!(d.normalized.client_ip.as_deref(), Some("192.168.1.1"));
        assert_eq!(d.normalized.body_size, 1024);
    }
}
