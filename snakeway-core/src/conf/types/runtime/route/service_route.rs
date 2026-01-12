use crate::conf::types::ServiceRouteSpec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceRouteConfig {
    /// Path prefix (longest-prefix match).
    pub path: String,

    pub service: String,

    pub allow_websocket: bool,
    pub ws_max_connections: Option<usize>,

    pub listener: String,
}

impl ServiceRouteConfig {
    pub fn new(service: String, listener: String, spec: ServiceRouteSpec) -> Self {
        Self {
            service,
            listener,
            path: spec.path,
            allow_websocket: spec.enable_websocket,
            ws_max_connections: spec.ws_max_connections,
        }
    }
}
