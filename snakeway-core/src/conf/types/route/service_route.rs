use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceRouteConfig {
    /// Path prefix (longest-prefix match).
    pub path: String,

    pub service: String,

    #[serde(default)]
    pub allow_websocket: bool,
    #[serde(default)]
    pub ws_idle_timeout_ms: u64,
    #[serde(default)]
    pub ws_max_connections: u64,
}
