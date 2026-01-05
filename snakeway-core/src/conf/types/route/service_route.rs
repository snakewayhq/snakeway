use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceRouteConfig {
    /// Path prefix (longest-prefix match).
    pub path: String,

    pub service: String,

    #[serde(default)]
    pub allow_websocket: bool,
    pub ws_max_connections: Option<usize>,
}
