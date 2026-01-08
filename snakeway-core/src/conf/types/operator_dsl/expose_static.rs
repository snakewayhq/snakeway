use crate::conf::types::{
    CircuitBreakerConfig, HealthCheckConfig, LoadBalancingStrategy, StaticCachePolicy,
    StaticFileConfig, TlsConfig,
};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default)]
pub struct ExposeStaticConfig {
    pub addr: String,
    pub tls: Option<TlsConfig>,
    pub routes: Vec<ExposeStaticRouteConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ExposeStaticRouteConfig {
    pub path: String,
    pub file_dir: PathBuf,
    pub index: Option<String>,
    pub directory_listing: bool,
    pub static_config: StaticFileConfig,
    pub cache_policy: StaticCachePolicy,
}
