use crate::conf::types::OnNoPeerAddrSpec;
use crate::conf::types::specification::ConnectionFilterSpec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct ConnectionFilterConfig {
    pub cidr_allow: Vec<String>,
    pub cidr_deny: Vec<String>,
    pub on_no_peer_addr: OnNoPeerAddr,
    pub ip_family_ipv4: bool,
    pub ip_family_ipv6: bool,
}

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub enum OnNoPeerAddr {
    #[default]
    Allow,
    Deny,
}

impl From<ConnectionFilterSpec> for ConnectionFilterConfig {
    fn from(spec: ConnectionFilterSpec) -> Self {
        Self {
            cidr_allow: spec.cidr.allow,
            cidr_deny: spec.cidr.deny,
            on_no_peer_addr: spec.on_no_peer_addr.into(),
            ip_family_ipv4: spec.ip_family.ipv4,
            ip_family_ipv6: spec.ip_family.ipv6,
        }
    }
}

impl From<OnNoPeerAddrSpec> for OnNoPeerAddr {
    fn from(on_no_peer_addr: OnNoPeerAddrSpec) -> Self {
        match on_no_peer_addr {
            OnNoPeerAddrSpec::Allow => OnNoPeerAddr::Allow,
            OnNoPeerAddrSpec::Deny => OnNoPeerAddr::Deny,
        }
    }
}
