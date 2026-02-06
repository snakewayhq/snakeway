mod connection_filter;
mod redirect;

pub use connection_filter::*;
pub use redirect::*;

use crate::conf::resolution::ResolveError;
use crate::conf::types::specification::bind_interface::{BindInterfaceInput, BindInterfaceSpec};
use crate::conf::types::{Origin, TlsSpec};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct BindSpec {
    #[serde(skip)]
    pub origin: Origin,
    pub interface: BindInterfaceInput,
    pub port: u16,
    pub tls: Option<TlsSpec>,
    pub enable_http2: bool,
    pub redirect_http_to_https: Option<RedirectSpec>,
    pub connection_filter: Option<ConnectionFilterSpec>,
}

impl BindSpec {
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
