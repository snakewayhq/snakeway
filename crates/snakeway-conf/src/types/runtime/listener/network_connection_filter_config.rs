use crate::types::OnNoPeerAddrSpec;
use crate::types::specification::NetworkConnectionFilterSpec;
use ipnet::IpNet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct NetworkConnectionFilterConfig {
    pub cidr_allow: Vec<IpNet>,
    pub cidr_deny: Vec<IpNet>,
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

impl TryFrom<NetworkConnectionFilterSpec> for NetworkConnectionFilterConfig {
    type Error = String;

    fn try_from(spec: NetworkConnectionFilterSpec) -> Result<Self, Self::Error> {
        let cidr_allow = spec
            .cidr
            .allow
            .iter()
            .map(|c| {
                c.parse::<IpNet>()
                    .map_err(|e| format!("invalid CIDR in allow list '{}': {}", c, e))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let cidr_deny = spec
            .cidr
            .deny
            .iter()
            .map(|c| {
                c.parse::<IpNet>()
                    .map_err(|e| format!("invalid CIDR in deny list '{}': {}", c, e))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            cidr_allow,
            cidr_deny,
            on_no_peer_addr: spec.on_no_peer_addr.into(),
            ip_family_ipv4: spec.ip_family.ipv4,
            ip_family_ipv6: spec.ip_family.ipv6,
        })
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
