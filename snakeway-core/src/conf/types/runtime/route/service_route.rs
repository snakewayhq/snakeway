use crate::conf::types::ServiceRouteSpec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ServiceRouteConfig {
    /// Host names allowed to access this route.
    pub(crate) hosts: Vec<String>,

    /// Path prefix (longest-prefix match).
    pub(crate) path: String,

    pub(crate) service: String,

    pub(crate) allow_websocket: bool,
    pub(crate) ws_max_connections: Option<usize>,

    pub(crate) listener: String,
}

impl ServiceRouteConfig {
    pub(crate) fn new(service: &str, listener: &str, spec: ServiceRouteSpec) -> Self {
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
