use crate::conf::types::{ConnectionFilterConfig, OnNoPeerAddr};
use async_trait::async_trait;
use pingora::listeners::ConnectionFilter;
use std::net::{IpAddr, SocketAddr};

#[derive(Debug, Default, Clone)]
pub struct NetworkConnectionFilter {
    cidr_allow: Vec<IpAddr>,
    cidr_deny: Vec<IpAddr>,
    ip_family_ipv4: bool,
    ip_family_ipv6: bool,
    on_no_peer_addr: OnNoPeerAddr,
}

#[async_trait]
impl ConnectionFilter for NetworkConnectionFilter {
    async fn should_accept(&self, addr_opt: Option<&SocketAddr>) -> bool {
        let addr = match addr_opt {
            Some(a) => a,
            None => return matches!(self.on_no_peer_addr, OnNoPeerAddr::Allow),
        };

        let ip = addr.ip();

        match ip {
            IpAddr::V4(_) if !self.ip_family_ipv4 => return false,
            IpAddr::V6(_) if !self.ip_family_ipv6 => return false,
            _ => {}
        }

        if self.cidr_deny.iter().any(|d| d == &ip) {
            return false;
        }

        if !self.cidr_allow.is_empty() && !self.cidr_allow.iter().any(|a| a == &ip) {
            return false;
        }

        true
    }
}

impl From<ConnectionFilterConfig> for NetworkConnectionFilter {
    fn from(config: ConnectionFilterConfig) -> Self {
        Self {
            cidr_allow: config
                .cidr_allow
                .into_iter()
                .map(|s| {
                    s.parse().expect(
                        "connection_filter.cidr.allow must be validated before runtime construction",
                    )
                })
                .collect(),
            cidr_deny: config
                .cidr_deny
                .into_iter()
                .map(|s| {
                    s.parse().expect(
                        "connection_filter.cidr.deny must be validated before runtime construction",
                    )
                })
                .collect(),
            ip_family_ipv4: config.ip_family_ipv4,
            ip_family_ipv6: config.ip_family_ipv6,
            on_no_peer_addr: config.on_no_peer_addr,
        }
    }
}
