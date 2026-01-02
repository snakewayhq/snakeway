use crate::conf::types::{DeviceConfig, ListenerConfig, RouteConfig, ServerConfig, ServiceConfig};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeConfig {
    pub server: ServerConfig,
    pub listeners: Vec<ListenerConfig>,
    pub routes: Vec<RouteConfig>,
    pub services: HashMap<String, ServiceConfig>,
    pub devices: Vec<DeviceConfig>,
}
