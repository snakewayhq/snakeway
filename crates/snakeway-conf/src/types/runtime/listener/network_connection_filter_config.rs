use crate::types::{NetworkConnectionFilterSpec, OnNoPeerAddrSpec};
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
    use crate::types::{CidrSpec, IpFamilySpec, OnNoPeerAddrSpec};

    #[test]
    fn valid_cidr_parsed() {
        // Arrange
        let spec = NetworkConnectionFilterSpec {
            cidr: CidrSpec {
                allow: vec!["10.0.0.0/8".to_string()],
                deny: vec!["192.168.0.0/16".to_string()],
            },
            ip_family: IpFamilySpec {
                ipv4: true,
                ipv6: false,
            },
            on_no_peer_addr: OnNoPeerAddrSpec::Allow,
        };

        // Act
        let config = NetworkConnectionFilterConfig::try_from(spec).unwrap();

        // Assert
        assert_eq!(config.cidr_allow.len(), 1);
        assert_eq!(config.cidr_allow[0], "10.0.0.0/8".parse().unwrap());
        assert_eq!(config.cidr_deny.len(), 1);
        assert_eq!(config.cidr_deny[0], "192.168.0.0/16".parse().unwrap());
    }

    #[test]
    fn invalid_cidr_fails() {
        // Arrange
        let spec = NetworkConnectionFilterSpec {
            cidr: CidrSpec {
                allow: vec!["not-a-cidr".to_string()],
                deny: vec![],
            },
            ip_family: IpFamilySpec::default(),
            on_no_peer_addr: OnNoPeerAddrSpec::default(),
        };

        // Act
        let result = NetworkConnectionFilterConfig::try_from(spec);

        // Assert
        assert!(result.is_err());
    }

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
