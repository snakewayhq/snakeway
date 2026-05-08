use crate::types::HclOrigin;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ServiceRouteSpec {
    #[serde(skip)]
    pub origin: HclOrigin,
    pub hosts: Vec<String>,
    pub path: String,
    #[serde(default)]
    pub enable_websocket: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_max_connections: Option<usize>,
}
