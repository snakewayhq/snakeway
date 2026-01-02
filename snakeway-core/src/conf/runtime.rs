use crate::conf::types::{DeviceConfig, ListenerConfig, RouteConfig, ServerConfig, ServiceConfig};
use crate::conf::validation::ValidationOutput;
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

pub struct ValidatedConfig {
    pub config: RuntimeConfig,
    pub validation: ValidationOutput,
}
