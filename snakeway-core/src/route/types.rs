use crate::conf::types::{StaticCachePolicy, StaticFileConfig};
use std::hash::{Hash, Hasher};
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
        static_config: StaticFileConfig,
        cache_policy: StaticCachePolicy,
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

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum RouteKind {
    Service,
    Static,
}

#[derive(Debug, Clone, Eq)]
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
