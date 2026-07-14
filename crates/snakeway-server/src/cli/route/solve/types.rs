use serde::Serialize;
use std::net::IpAddr;

/// A request representation decoupled from any server runtime (no Pingora session).
/// Used by `route solve` and tests.
pub struct SyntheticRequest {
    pub scheme: String,
    pub host: String,
    pub method: http::Method,
    pub path: String,
    pub query: Option<String>,
    pub client_ip: Option<IpAddr>,
    pub body_size: usize,
}

/// Options controlling deterministic upstream selection during solve.
pub struct RouteSolveOptions {
    pub lb_key: Option<String>,
    pub lb_index: Option<usize>,
    pub trace: bool,
    pub verbose: bool,
}

/// The complete result of a dry-run route solve.
#[derive(Serialize)]
pub struct RouteSolveDecision {
    pub matched_route: Option<String>,
    pub route_kind: Option<String>,
    pub upstream_service: Option<String>,
    pub selected_upstream: Option<String>,
    pub static_file_dir: Option<String>,
    pub rejection: Option<RouteSolveRejection>,
    pub normalized: RouteSolveNormalized,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<Vec<RouteSolveTraceStep>>,
}

#[derive(Serialize)]
pub struct RouteSolveRejection {
    pub stage: String,
    pub reason: String,
}

#[derive(Serialize)]
pub struct RouteSolveNormalized {
    pub scheme: String,
    pub host: String,
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub client_ip: Option<String>,
    pub body_size: usize,
}

#[derive(Serialize, Clone)]
pub struct RouteSolveTraceStep {
    pub stage: String,
    pub outcome: String,
    pub detail: String,
}
