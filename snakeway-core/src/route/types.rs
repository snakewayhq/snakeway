use crate::conf::types::{CachePolicy, CompressionOptions};
use http::HeaderMap;
use serde::Serialize;
use std::hash::{Hash, Hasher};
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum RouteRuntime {
    /// Forward request to upstream
    Service {
        id: RouteId,
        upstream: String,
        allow_websocket: bool,
        ws_max_connections: Option<usize>,
    },

    /// Serve files from the local filesystem
    Static {
        id: RouteId,
        path: String,
        file_dir: PathBuf,
        index: bool,
        directory_listing: bool,
        max_file_size: u64,
        static_config: CompressionOptions,
        cache_policy: CachePolicy,
    },
}

impl RouteRuntime {
    pub fn id(&self) -> &RouteId {
        match self {
            RouteRuntime::Service { id, .. } => id,
            RouteRuntime::Static { id, .. } => id,
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize)]
pub enum RouteKind {
    Service,
    Static,
}

#[derive(Debug, Clone, Eq, Serialize)]
pub struct RouteId {
    kind: RouteKind,
    path: Arc<str>,
    target: Arc<str>,
}

impl PartialEq for RouteId {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.path == other.path && self.target == other.target
    }
}

impl Hash for RouteId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.path.hash(state);
        self.target.hash(state);
    }
}

impl RouteId {
    pub fn service(path: &str, service: &str) -> Self {
        Self {
            kind: RouteKind::Service,
            path: Arc::from(path.trim_end_matches('/')),
            target: Arc::from(service),
        }
    }

    pub fn static_route(path: &str, file_dir: &str) -> Self {
        Self {
            kind: RouteKind::Static,
            path: Arc::from(path.trim_end_matches('/')),
            target: Arc::from(file_dir),
        }
    }

    /// Stable string form for logging / admin APIs
    pub fn as_str(&self) -> String {
        let kind = match self.kind {
            RouteKind::Service => "service",
            RouteKind::Static => "static",
        };

        format!("{kind}:{}:{}", self.path, self.target)
    }

    pub fn kind(&self) -> RouteKind {
        self.kind
    }
}

// ---------------------------------------------------------------------------
// Synthetic request / dry-run solve types
// ---------------------------------------------------------------------------

/// A request representation decoupled from any server runtime (no Pingora session).
/// Used by `route solve` and tests.
pub struct SyntheticRequest {
    pub scheme: String,
    pub host: String,
    pub method: http::Method,
    pub path: String,
    pub query: Option<String>,
    pub headers: HeaderMap,
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
