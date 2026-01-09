use crate::conf::types::{CircuitBreakerConfig, HealthCheckConfig, LoadBalancingStrategy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct ExposeServiceConfig {
    #[serde(default)]
    pub load_balancing_strategy: LoadBalancingStrategy,
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

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ExposeBackendConfig {
    pub tcp: Option<TcpConfig>,
    pub unix: Option<UnixConfig>,
    #[serde(default = "default_weight")]
    pub weight: u32,
}

fn default_weight() -> u32 {
    1
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TcpConfig {
    pub addr: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UnixConfig {
    pub sock: String,
}
