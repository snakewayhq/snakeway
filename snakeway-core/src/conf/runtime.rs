use crate::conf::types::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub server: ServerConfig,
    pub routes: Vec<RouteConfig>,
    pub services: HashMap<String, ServiceConfig>,
    pub devices: Vec<DeviceConfig>,
}
