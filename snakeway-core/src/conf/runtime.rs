use crate::conf::types::*;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeConfig {
    pub server: ServerConfig,
    pub routes: Vec<RouteConfig>,
    pub services: HashMap<String, ServiceConfig>,
    pub devices: Vec<DeviceConfig>,
}
