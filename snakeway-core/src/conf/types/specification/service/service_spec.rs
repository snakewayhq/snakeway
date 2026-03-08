use crate::conf::resolution::ResolveError;
use crate::conf::types::{CircuitBreakerSpec, HealthCheckSpec, Origin};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default, Serialize)]
pub(crate) struct ServiceSpec {
    #[serde(skip)]
    pub(crate) origin: Origin,
    #[serde(default)]
    pub(crate) load_balancing_strategy: LoadBalancingStrategySpec,
    pub(crate) routes: Vec<ServiceRouteSpec>,
    pub(crate) upstreams: Vec<UpstreamSpec>,
    pub(crate) health_check: Option<HealthCheckSpec>,
    pub(crate) circuit_breaker: Option<CircuitBreakerSpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LoadBalancingStrategySpec {
    #[default]
    Failover,
    RoundRobin,
    RequestPressure,
    StickyHash,
    Random,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub(crate) struct ServiceRouteSpec {
    #[serde(skip)]
    pub(crate) origin: Origin,
    pub(crate) hosts: Vec<String>,
    pub(crate) path: String,
    #[serde(default)]
    pub(crate) enable_websocket: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ws_max_connections: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub(crate) struct UpstreamSpec {
    #[serde(skip)]
    pub(crate) origin: Origin,
    pub(crate) endpoint: Option<EndpointSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sock: Option<String>,
    #[serde(default = "default_weight")]
    pub(crate) weight: u32,
}
fn default_weight() -> u32 {
    1
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub(crate) enum HostSpec {
    Ip(std::net::IpAddr),
    Hostname(String),
}

impl fmt::Display for HostSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostSpec::Ip(ip) => write!(f, "{ip}"),
            HostSpec::Hostname(name) => write!(f, "{}", name.to_lowercase()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub(crate) struct EndpointSpec {
    pub(crate) host: HostSpec,
    pub(crate) port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tls: Option<EndpointTlsSpec>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub(crate) struct EndpointTlsSpec {
    pub(crate) sni: String,
    pub(crate) verify: bool,
    pub(crate) ca_file: Option<PathBuf>,
}

impl EndpointSpec {
    pub(crate) fn resolve(&self) -> Result<SocketAddr, ResolveError> {
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
