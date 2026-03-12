use crate::types::ServiceRouteSpec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceRouteConfig {
    /// Host names allowed to access this route.
    pub hosts: Vec<String>,

    /// Path prefix (longest-prefix match).
    pub path: String,

    pub service: String,

    pub allow_websocket: bool,
    pub ws_max_connections: Option<usize>,

    pub listener: String,
}

impl ServiceRouteConfig {
    pub fn new(service: &str, listener: &str, spec: ServiceRouteSpec) -> Self {
        Self {
            service: service.to_string(),
            listener: listener.to_string(),
            hosts: spec.hosts,
            path: spec.path,
            allow_websocket: spec.enable_websocket,
            ws_max_connections: spec.ws_max_connections,
        }
    }
}
