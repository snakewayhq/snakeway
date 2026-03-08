use crate::conf::resolution::ResolveError;
use crate::conf::types::specification::bind_interface::{BindInterfaceInput, BindInterfaceSpec};
use crate::conf::types::{Origin, TlsTerminationSpec};
use crate::conf::validation::ConfigError;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Deserialize, Default, Serialize)]
pub(crate) struct BindAdminSpec {
    #[serde(skip)]
    pub(crate) origin: Origin,
    pub(crate) interface: BindInterfaceInput,
    pub(crate) port: u16,
    pub(crate) tls: TlsTerminationSpec,
}

impl BindAdminSpec {
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
