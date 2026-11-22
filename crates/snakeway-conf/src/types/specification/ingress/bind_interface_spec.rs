use crate::validation::ConfigError;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BindInterfaceSpec {
    /// 127.0.0.1 / ::1
    #[default]
    Loopback,
    /// 0.0.0.0 / ::
    All,
    /// Custom IP address defined by an operator.
    Ip(std::net::IpAddr),
}

/// Whether `ip` is a link-local address: `169.254.0.0/16` for IPv4 and
/// `fe80::/10` for IPv6.
pub(crate) fn is_link_local(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_unicast_link_local(),
    }
}

impl BindInterfaceSpec {
    pub fn as_ip(&self) -> IpAddr {
        match self {
            BindInterfaceSpec::Loopback => IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            BindInterfaceSpec::All => IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
            BindInterfaceSpec::Ip(ip) => *ip,
        }
    }

    pub fn socket_address_literal(&self, port: u16) -> String {
        format!("{}:{}", self.as_ip(), port)
    }
}

/// Interfaces are written as a keyword (`"loopback"`, `"all"`) or an IP
/// address literal.
impl TryFrom<&str> for BindInterfaceSpec {
    type Error = ConfigError;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        match input {
            "loopback" => Ok(BindInterfaceSpec::Loopback),
            "all" => Ok(BindInterfaceSpec::All),
            _ => {
                let ip = IpAddr::from_str(input)
                    .map_err(|_| ConfigError::InvalidBindIpString(input.to_string()))?;
                Ok(BindInterfaceSpec::Ip(ip))
            }
        }
    }
}
