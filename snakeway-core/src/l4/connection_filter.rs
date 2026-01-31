use async_trait::async_trait;
use pingora::listeners::ConnectionFilter;
use std::net::{IpAddr, SocketAddr};

#[derive(Debug, Default, Clone)]
pub struct L4ConnectionFilter {
    cidr_allow: Vec<IpAddr>,
    cidr_deny: Vec<IpAddr>,
    ip_family_ipv4: bool,
    ip_family_ipv6: bool,
    on_no_peer_addr: String,
}

impl L4ConnectionFilter {
    pub fn new(
        cidr_allow: Vec<IpAddr>,
        cidr_deny: Vec<IpAddr>,
        ip_family_ipv4: bool,
        ip_family_ipv6: bool,
    ) -> Self {
        Self {
            cidr_allow,
            cidr_deny,
            ip_family_ipv4,
            ip_family_ipv6,
            on_no_peer_addr: "allow".to_string(),
        }
    }
}
#[async_trait]
impl ConnectionFilter for L4ConnectionFilter {
    async fn should_accept(&self, addr_opt: Option<&SocketAddr>) -> bool {
        let addr = match addr_opt {
            Some(a) => a,
            None => return self.on_no_peer_addr == "allow",
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
