use crate::cli::route::solve::types::{
    RouteSolveDecision, RouteSolveNormalized, RouteSolveOptions, RouteSolveRejection,
    RouteSolveTraceStep, SyntheticRequest,
};
use crate::route::{RouteEntry, RouteRuntime};
use crate::runtime::{RuntimeState, ServiceRuntime, UpstreamRuntime};

/// Deterministic, side-effect-free route resolution.
///
/// Walks the same router matching path the proxy uses, then selects an
/// upstream according to the supplied [`RouteSolveOptions`].
pub fn solve(
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
fn fnv1a_hash(data: &[u8]) -> u32 {
    const FNV_OFFSET: u32 = 2166136261;
    const FNV_PRIME: u32 = 16777619;
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
