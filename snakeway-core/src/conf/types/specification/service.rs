use crate::conf::types::{CircuitBreakerConfig, HealthCheckConfig, LoadBalancingStrategy, Origin};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct ServiceSpec {
    #[serde(skip)]
    pub origin: Origin,
    #[serde(default)]
    pub load_balancing_strategy: LoadBalancingStrategy,
    pub routes: Vec<ServiceRouteSpec>,
    pub upstreams: Vec<UpstreamSpec>,
    pub health_check: Option<HealthCheckConfig>,
    pub circuit_breaker: Option<CircuitBreakerConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServiceRouteSpec {
    #[serde(skip)]
    pub origin: Origin,
    pub path: String,
    #[serde(default)]
    pub enable_websocket: bool,
    pub ws_max_connections: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct UpstreamSpec {
    #[serde(skip)]
    pub origin: Origin,
    pub addr: Option<String>,
    pub sock: Option<String>,
    pub sock_options: Option<UnixTransportOptionsSpec>,
    #[serde(default = "default_weight")]
    pub weight: u32,
}
fn default_weight() -> u32 {
    1
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct UnixTransportOptionsSpec {
    pub use_tls: bool,
    pub sni: String,
}
