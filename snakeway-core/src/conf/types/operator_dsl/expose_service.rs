use crate::conf::types::{
    CircuitBreakerConfig, HealthCheckConfig, LoadBalancingStrategy, StaticCachePolicy,
    StaticFileConfig, TlsConfig,
};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default)]
pub struct ExposeServiceConfig {
    pub addr: String,
    pub tls: Option<TlsConfig>,
    pub enable_http2: bool,
    pub strategy: LoadBalancingStrategy,
    pub routes: Vec<ExposeRouteConfig>,
    pub backends: Vec<ExposeBackendConfig>,
    pub health_check: Option<HealthCheckConfig>,
    pub circuit_breaker: Option<CircuitBreakerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ExposeRouteConfig {
    pub path: String,
    #[serde(default)]
    pub enable_websocket: bool,
    pub ws_max_connections: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ExposeBackendConfig {
    pub tcp: Option<TcpConfig>,
    pub unix: Option<UnixConfig>,
    pub weight: u32,
}

#[derive(Debug, Deserialize)]
pub struct TcpConfig {
    pub addr: String,
}

#[derive(Debug, Deserialize)]
pub struct UnixConfig {
    pub sock: String,
}
