use crate::conf::resolution::ResolveError;
use crate::conf::types::{CircuitBreakerConfig, HealthCheckConfig, Origin};
use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, ToSocketAddrs};

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct ServiceSpec {
    #[serde(skip)]
    pub origin: Origin,
    #[serde(default)]
    pub load_balancing_strategy: LoadBalancingStrategySpec,
    pub routes: Vec<ServiceRouteSpec>,
    pub upstreams: Vec<UpstreamSpec>,
    pub health_check: Option<HealthCheckConfig>,
    pub circuit_breaker: Option<CircuitBreakerConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalancingStrategySpec {
    #[default]
    Failover,
    RoundRobin,
    RequestPressure,
    StickyHash,
    Random,
}

#[derive(Debug, Deserialize, Serialize, Default)]
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
    pub endpoint: Option<EndpointSpec>,
    pub sock: Option<String>,
    #[serde(default = "default_weight")]
    pub weight: u32,
}
fn default_weight() -> u32 {
    1
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum HostSpec {
    Ip(std::net::IpAddr),
    Hostname(String),
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct EndpointSpec {
    pub host: HostSpec,
    pub port: u16,
}

impl EndpointSpec {
    pub fn resolve(&self) -> Result<SocketAddr, ResolveError> {
        let ip = match &self.host {
            HostSpec::Ip(ip) => *ip,
            HostSpec::Hostname(name) => {
                let mut addrs = (name.as_str(), self.port)
                    .to_socket_addrs()
                    .map_err(|_| ResolveError::DnsFailed(name.clone()))?;

                addrs
                    .next()
                    .ok_or_else(|| ResolveError::NoAddresses(name.clone()))?
                    .ip()
            }
        };

        Ok(SocketAddr::new(ip, self.port))
    }
}
