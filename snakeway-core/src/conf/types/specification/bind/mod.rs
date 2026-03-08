mod connection_rate_limiting_filter;
mod network_connection_filter;
mod redirect;
mod tls_termination;

pub(crate) use connection_rate_limiting_filter::*;
pub(crate) use network_connection_filter::*;
pub(crate) use redirect::*;
pub(crate) use tls_termination::*;

use crate::conf::resolution::ResolveError;
use crate::conf::types::Origin;
use crate::conf::types::specification::bind_interface::{BindInterfaceInput, BindInterfaceSpec};
use crate::conf::validation::ConfigError;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub(crate) struct BindSpec {
    #[serde(skip)]
    pub(crate) origin: Origin,
    pub(crate) interface: BindInterfaceInput,
    pub(crate) port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tls: Option<TlsTerminationSpec>,
    pub(crate) enable_http2: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) redirect_http_to_https: Option<RedirectSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) connection_filter: Option<NetworkConnectionFilterSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) connection_rate_limiting_filter: Option<ConnectionRateLimitingFilterSpec>,
}

impl BindSpec {
    pub(crate) fn resolve(&self) -> Result<SocketAddr, ResolveError> {
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
