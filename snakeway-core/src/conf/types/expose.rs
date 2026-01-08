/*
[[expose_redirect]]
addr = "0.0.0.0:80"
to = "https://{host}{uri}"
status = 301

[expose_service]
addr = "127.0.0.1:8443"
tls = { cert = "./integration-tests/certs/server.pem", key = "./integration-tests/certs/server.key" }
enable_http2 = true
strategy = "round_robin"

[expose_service.health_check]
enable = false
failure_threshold = 3
unhealthy_cooldown_seconds = 10

[expose_service.circuit_breaker]
enable_auto_recovery = false
failure_threshold = 3
open_duration_ms = 10000
half_open_max_requests = 1
success_threshold = 2
count_http_5xx_as_failure = false

[[expose_service.routes]]
path = "/api"

[[expose_service.routes]]
path = "/ws"
enable_websocket = true
ws_max_connections = 10_000

[[expose_service.backends]]
tcp = { addr = "127.0.0.1:3443" }
weight = 1

[[expose_service.backends]]
tcp = { addr = "127.0.0.1:3444" }
weight = 1

[[expose_service.backends]]
unix = { sock = "/tmp/snakeway-http" }
weight = 1


*/
use crate::conf::types::{
    CircuitBreakerConfig, HealthCheckConfig, LoadBalancingStrategy, TlsConfig,
};
use serde::Deserialize;

pub enum ExposeConfig {
    Redirect(ExposeRedirectConfig),
    Service(ExposeServiceConfig),
    StaticRoute(ExposeStaticConfig),
}

#[derive(Debug, Deserialize, Default)]
pub struct ExposeRedirectConfig {
    pub addr: String,
    pub to: String,
    pub status: u16,
}

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
    pub weight: usize,
}

#[derive(Debug, Deserialize)]
pub struct TcpConfig {
    pub addr: String,
}

#[derive(Debug, Deserialize)]
pub struct UnixConfig {
    pub sock: String,
}

/*
[expose_static]
addr = "127.0.0.1:8443"
tls = { cert = "./integration-tests/certs/server.pem", key = "./integration-tests/certs/server.key" }

[[expose_static.routes]]
path = "/assets"
file_dir = "/var/www/html/"
index = "index.html"
directory_listing = false
*/

#[derive(Debug, Deserialize, Default)]
pub struct ExposeStaticConfig {
    pub addr: String,
    pub tls: Option<TlsConfig>,
    pub routes: Vec<ExposeStaticRouteConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ExposeStaticRouteConfig {
    pub path: String,
    pub file_dir: String,
    pub index: Option<String>,
    pub directory_listing: bool,
}
