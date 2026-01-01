use crate::conf::types::route::RouteKind;

pub struct NormalizedRoute {
    pub path: String,
    pub kind: RouteKind,
    pub allow_websocket: bool,
    pub ws_idle_timeout_ms: Option<u64>,
    pub ws_max_connections: Option<u64>,
}
