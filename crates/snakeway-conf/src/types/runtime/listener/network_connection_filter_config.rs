use crate::types::OnNoPeerAddrSpec;
use ipnet::IpNet;
use o2o::o2o;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize, Clone, PartialEq)]
pub struct NetworkConnectionFilterConfig {
    pub cidr_allow: Vec<IpNet>,
    pub cidr_deny: Vec<IpNet>,
    pub on_no_peer_addr: OnNoPeerAddr,
    pub ip_family_ipv4: bool,
    pub ip_family_ipv6: bool,
}

#[derive(o2o, Debug, Deserialize, Default, Serialize, Clone, PartialEq)]
#[from_owned(OnNoPeerAddrSpec)]
pub enum OnNoPeerAddr {
    #[default]
    Allow,
    Deny,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_no_peer_addr_allow() {
        // Arrange
        let spec = OnNoPeerAddrSpec::Allow;

        // Act
        let config: OnNoPeerAddr = spec.into();

        // Assert
        assert!(matches!(config, OnNoPeerAddr::Allow));
    }

    #[test]
    fn on_no_peer_addr_deny() {
        // Arrange
        let spec = OnNoPeerAddrSpec::Deny;

        // Act
        let config: OnNoPeerAddr = spec.into();

        // Assert
        assert!(matches!(config, OnNoPeerAddr::Deny));
    }
}
