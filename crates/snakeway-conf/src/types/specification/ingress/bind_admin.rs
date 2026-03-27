use crate::resolution::ResolveError;
use crate::types::specification::ingress::bind_interface::{BindInterfaceInput, BindInterfaceSpec};
use crate::types::{Origin, TlsTerminationSpec};
use crate::validation::ConfigError;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct BindAdminSpec {
    #[serde(skip)]
    pub origin: Origin,
    pub interface: BindInterfaceInput,
    pub port: u16,
    pub tls: TlsTerminationSpec,
}

impl BindAdminSpec {
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
