use crate::conf::types::{ServiceRouteConfig, StaticRouteConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) enum RouteConfig {
    Service(ServiceRouteConfig),
    Static(StaticRouteConfig),
}

impl RouteConfig {
    pub(crate) fn hosts(&self) -> Vec<String> {
        match self {
            RouteConfig::Service(cfg) => cfg.hosts.clone(),
            RouteConfig::Static(cfg) => cfg.hosts.clone(),
        }
    }
    pub(crate) fn path(&self) -> &str {
        match self {
            RouteConfig::Service(cfg) => &cfg.path,
            RouteConfig::Static(cfg) => &cfg.path,
        }
    }

    pub(crate) fn listener(&self) -> &str {
        match self {
            RouteConfig::Service(cfg) => &cfg.listener,
            RouteConfig::Static(cfg) => &cfg.listener,
        }
    }

    pub(crate) fn set_listener(&mut self, listener: String) {
        match self {
            RouteConfig::Service(cfg) => cfg.listener = listener,
            RouteConfig::Static(cfg) => cfg.listener = listener,
        }
    }
}
