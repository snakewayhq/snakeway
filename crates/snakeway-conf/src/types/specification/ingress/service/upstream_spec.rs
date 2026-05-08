use crate::resolution::ResolveError;
use crate::types::HclOrigin;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct UpstreamSpec {
    #[serde(skip)]
    pub origin: HclOrigin,
    pub endpoint: Option<EndpointSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sock: Option<String>,
    #[serde(default = "default_weight")]
    pub weight: u32,
}
fn default_weight() -> u32 {
    1
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum HostSpec {
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
pub struct EndpointSpec {
    pub host: HostSpec,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<EndpointTlsSpec>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct EndpointTlsSpec {
    pub sni: String,
    pub verify: bool,
    pub ca_file: Option<PathBuf>,
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
