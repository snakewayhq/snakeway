use crate::conf::types::{CircuitBreakerConfig, HealthCheckConfig, LoadBalancingStrategy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct ExposeServiceConfig {
    pub strategy: LoadBalancingStrategy,
    pub routes: Vec<ExposeRouteConfig>,
    pub backends: Vec<ExposeBackendConfig>,
    pub health_check: Option<HealthCheckConfig>,
    pub circuit_breaker: Option<CircuitBreakerConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExposeRouteConfig {
    pub path: String,
    #[serde(default)]
    pub enable_websocket: bool,
    pub ws_max_connections: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExposeBackendConfig {
    pub tcp: Option<TcpConfig>,
    pub unix: Option<UnixConfig>,
    pub weight: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TcpConfig {
    pub addr: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UnixConfig {
    pub sock: String,
}
