use crate::conf::types::{ServiceRouteConfig, StaticRouteConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum RouteConfig {
    Service(ServiceRouteConfig),
    Static(StaticRouteConfig),
}

impl RouteConfig {
    pub fn path(&self) -> &str {
        match self {
            RouteConfig::Service(cfg) => &cfg.path,
            RouteConfig::Static(cfg) => &cfg.path,
        }
    }

    pub fn listener(&self) -> &str {
        match self {
            RouteConfig::Service(cfg) => &cfg.listener,
            RouteConfig::Static(cfg) => &cfg.listener,
        }
    }
}
