use crate::conf::resolution::ResolveError;
use crate::conf::types::specification::bind_interface::{BindInterfaceInput, BindInterfaceSpec};
use crate::conf::types::{Origin, TlsSpec};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct BindAdminSpec {
    #[serde(skip)]
    pub origin: Origin,
    pub interface: BindInterfaceInput,
    pub port: u16,
    pub tls: TlsSpec,
}

impl BindAdminSpec {
    pub fn resolve(&self) -> Result<SocketAddr, ResolveError> {
        let interface: BindInterfaceSpec = self
            .interface
            .clone()
            .try_into()
            .expect("BindInterfaceSpec must be validated before resolve()");

        let ip = match interface {
            BindInterfaceSpec::Loopback => std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            BindInterfaceSpec::All => std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
            BindInterfaceSpec::Ip(ip) => ip,
        };

        Ok(SocketAddr::new(ip, self.port))
    }
}
