use serde::Serialize;
use std::net::IpAddr;

/// A request representation decoupled from any server runtime (no Pingora session).
/// Used by `route solve` and tests.
pub(crate) struct SyntheticRequest {
    pub(crate) scheme: String,
    pub(crate) host: String,
    pub(crate) method: http::Method,
    pub(crate) path: String,
    pub(crate) query: Option<String>,
    pub(crate) client_ip: Option<IpAddr>,
    pub(crate) body_size: usize,
}

/// Options controlling deterministic upstream selection during solve.
pub(crate) struct RouteSolveOptions {
    pub(crate) lb_key: Option<String>,
    pub(crate) lb_index: Option<usize>,
    pub(crate) trace: bool,
    pub(crate) verbose: bool,
}

/// The complete result of a dry-run route solve.
#[derive(Serialize)]
pub(crate) struct RouteSolveDecision {
    pub(crate) matched_route: Option<String>,
    pub(crate) route_kind: Option<String>,
    pub(crate) upstream_service: Option<String>,
    pub(crate) selected_upstream: Option<String>,
    pub(crate) static_file_dir: Option<String>,
    pub(crate) rejection: Option<RouteSolveRejection>,
    pub(crate) normalized: RouteSolveNormalized,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) trace: Option<Vec<RouteSolveTraceStep>>,
}

#[derive(Serialize)]
pub(crate) struct RouteSolveRejection {
    pub(crate) stage: String,
    pub(crate) reason: String,
}

#[derive(Serialize)]
pub(crate) struct RouteSolveNormalized {
    pub(crate) scheme: String,
    pub(crate) host: String,
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) query: Option<String>,
    pub(crate) client_ip: Option<String>,
    pub(crate) body_size: usize,
}

#[derive(Serialize, Clone)]
pub(crate) struct RouteSolveTraceStep {
    pub(crate) stage: String,
    pub(crate) outcome: String,
    pub(crate) detail: String,
}
