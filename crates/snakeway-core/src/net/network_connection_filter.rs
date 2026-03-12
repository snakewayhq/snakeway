use crate::net::CidrCollection;
use async_trait::async_trait;
use pingora::listeners::ConnectionFilter;
use snakeway_conf::types::{NetworkConnectionFilterConfig, OnNoPeerAddr};
use std::fmt::Debug;
use std::net::{IpAddr, SocketAddr};
use tracing::debug;

#[derive(Debug, Default, Clone)]
pub(crate) struct NetworkConnectionFilter {
    pub(crate) cidr_allow: CidrCollection,
    pub(crate) cidr_deny: CidrCollection,
    pub(crate) ip_family_ipv4: bool,
    pub(crate) ip_family_ipv6: bool,
    pub(crate) on_no_peer_addr: OnNoPeerAddr,
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
            IpAddr::V4(_) if !self.ip_family_ipv4 => {
                debug!(%client_ip, "Connection rejected as IPv4 is disabled in filter config");
                return false;
            }
            IpAddr::V6(_) if !self.ip_family_ipv6 => {
                debug!(%client_ip, "Connection rejected as IPv6 is disabled in filter config");
                return false;
            }
            _ => {}
        }

        // Any explicit deny entry takes precedence.
        // If there is at least one deny entry and that client IP is in the deny list, reject.
        if !self.cidr_deny.is_empty() && self.cidr_deny.contains(client_ip) {
            debug!(%client_ip, "Connection rejected by CIDR deny list");
            return false;
        }

        // Finally, if the allow list is empty OR the client IP is in the allow list, accept.
        if self.cidr_allow.is_empty() || self.cidr_allow.contains(client_ip) {
            true
        } else {
            debug!(%client_ip, "Connection rejected by CIDR allow list");
            false
        }
    }
}

impl From<NetworkConnectionFilterConfig> for NetworkConnectionFilter {
    fn from(config: NetworkConnectionFilterConfig) -> Self {
        Self {
            cidr_allow: CidrCollection::new(&config.cidr_allow),
            cidr_deny: CidrCollection::new(&config.cidr_deny),
            ip_family_ipv4: config.ip_family_ipv4,
            ip_family_ipv6: config.ip_family_ipv6,
            on_no_peer_addr: config.on_no_peer_addr,
        }
    }
}
