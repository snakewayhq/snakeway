mod connection_rate_limiting_filter;
mod network_connection_filter;
mod redirect;
mod tls_termination;

pub use connection_rate_limiting_filter::*;
pub use network_connection_filter::*;
pub use redirect::*;
pub use tls_termination::*;

use crate::resolution::ResolveError;
use crate::types::Origin;
use crate::types::specification::bind_interface::{BindInterfaceInput, BindInterfaceSpec};
use crate::validation::ConfigError;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct BindSpec {
    #[serde(skip)]
    pub origin: Origin,
    pub interface: BindInterfaceInput,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsTerminationSpec>,
    pub enable_http2: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_http_to_https: Option<RedirectSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_filter: Option<NetworkConnectionFilterSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_rate_limiting_filter: Option<ConnectionRateLimitingFilterSpec>,
}

impl BindSpec {
    pub fn resolve(&self) -> Result<SocketAddr, ResolveError> {
        let interface: BindInterfaceSpec = self
            .interface
            .clone()
            .try_into()
            .map_err(|e: ConfigError| ResolveError::InvalidInterface(e.to_string()))?;

        let ip = match interface {
            BindInterfaceSpec::Loopback => std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            BindInterfaceSpec::All => std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
            BindInterfaceSpec::Ip(ip) => ip,
        };
        Ok(SocketAddr::new(ip, self.port))
    }
}
