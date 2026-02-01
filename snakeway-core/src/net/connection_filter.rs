use crate::conf::types::{ConnectionFilterConfig, OnNoPeerAddr};
use crate::net::{is_addr_allowed, is_addr_denied};
use async_trait::async_trait;
use ipnet::IpNet;
use pingora::listeners::ConnectionFilter;
use std::net::{IpAddr, SocketAddr};

#[derive(Debug, Default, Clone)]
pub struct NetworkConnectionFilter {
    cidr_allow: Vec<IpNet>,
    cidr_deny: Vec<IpNet>,
    ip_family_ipv4: bool,
    ip_family_ipv6: bool,
    on_no_peer_addr: OnNoPeerAddr,
}

#[async_trait]
impl ConnectionFilter for NetworkConnectionFilter {
    async fn should_accept(&self, addr_opt: Option<&SocketAddr>) -> bool {
        // If we do not have a peer address, defer to the configured default behavior.
        let addr = match addr_opt {
            Some(a) => a,
            None => return matches!(self.on_no_peer_addr, OnNoPeerAddr::Allow),
        };

        // Check IP family gating before any allow/deny list checks.
        let client_ip = addr.ip();

        match client_ip {
            IpAddr::V4(_) if !self.ip_family_ipv4 => return false,
            IpAddr::V6(_) if !self.ip_family_ipv6 => return false,
            _ => {}
        }

        // Any explicit deny entry takes precedence.
        if is_addr_denied(client_ip, &self.cidr_deny) {
            return false;
        }

        // When an allow list is configured, only addresses on it pass.
        if !is_addr_allowed(client_ip, &self.cidr_allow) {
            return false;
        }

        // Passed all configured checks.
        true
    }
}

impl From<ConnectionFilterConfig> for NetworkConnectionFilter {
    fn from(config: ConnectionFilterConfig) -> Self {
        Self {
            cidr_allow: config.cidr_allow,
            cidr_deny: config.cidr_deny,
            ip_family_ipv4: config.ip_family_ipv4,
            ip_family_ipv6: config.ip_family_ipv6,
            on_no_peer_addr: config.on_no_peer_addr,
        }
    }
}
