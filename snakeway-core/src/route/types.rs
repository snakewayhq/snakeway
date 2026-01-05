use crate::conf::types::{StaticCachePolicy, StaticFileConfig};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum RouteRuntime {
    /// Forward request to upstream
    Service {
        id: RouteId,
        upstream: String,
        allow_websocket: bool,
        ws_max_connections: Option<usize>,
        ws_idle_timeout_ms: Option<usize>,
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

/// RouteId format:
/// service:{path}:{service}
/// static:{path}:{file_dir}
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct RouteId(Arc<str>);

impl RouteId {
    pub fn new(s: impl Into<Arc<str>>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn from_config(kind: &str, path: &str, target: &str) -> Self {
        let normalized_path = path.trim_end_matches('/');
        let id = format!("{}:{}:{}", kind, normalized_path, target);
        Self::new(id)
    }
}

pub fn canonicalize_dir(dir: &Path) -> String {
    let path_buf = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let result = path_buf.to_string_lossy();
    result.to_string()
}
