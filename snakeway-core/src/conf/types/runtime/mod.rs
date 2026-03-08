pub(crate) mod device;
pub(crate) mod listener;
pub(crate) mod route;
pub(crate) mod server;
pub(crate) mod service;

pub(crate) use device::*;
pub(crate) use listener::*;
pub(crate) use route::*;
pub(crate) use server::*;
pub(crate) use service::*;

use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeConfig {
    pub(crate) server: ServerConfig,
    pub(crate) listeners: Vec<ListenerConfig>,
    pub(crate) routes: Vec<RouteConfig>,
    pub(crate) services: HashMap<String, ServiceConfig>,
    pub(crate) devices: Vec<DeviceConfig>,
}
