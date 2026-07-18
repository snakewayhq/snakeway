pub mod device;
pub mod listener;
pub mod route;
pub mod server;
pub mod service;

pub use device::*;
pub use listener::*;
pub use route::*;
pub use server::*;
pub use service::*;

use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeConfig {
    pub server: ServerConfig,
    pub listeners: Vec<ListenerConfig>,
    pub routes: Vec<RouteConfig>,
    pub services: HashMap<String, ServiceConfig>,
    pub devices: Vec<DeviceConfig>,
}
