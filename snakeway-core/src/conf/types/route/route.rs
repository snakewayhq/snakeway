use crate::conf::types::route::RouteKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RouteConfig {
    /// Path prefix (longest-prefix match).
    pub path: String,

    /// Target route.
    pub kind: RouteKind,

    pub allow_websocket: bool,
    pub ws_idle_timeout_ms: Option<u64>,
    pub ws_max_connections: Option<u64>,
}
