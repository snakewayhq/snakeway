use crate::types::{NetworkConnectionFilterSpec, ON_NO_PEER_ADDR_DENY};
use ipnet::IpNet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize, Clone, PartialEq)]
pub struct NetworkConnectionFilterConfig {
    pub cidr_allow: Vec<IpNet>,
    pub cidr_deny: Vec<IpNet>,
    pub on_no_peer_addr: OnNoPeerAddr,
    pub ip_family_ipv4: bool,
    pub ip_family_ipv6: bool,
}

impl TryFrom<&NetworkConnectionFilterSpec> for NetworkConnectionFilterConfig {
    type Error = String;

    fn try_from(spec: &NetworkConnectionFilterSpec) -> Result<Self, Self::Error> {
        let cidr_allow = spec
            .cidr
            .value
            .allow
            .iter()
            .map(|c| {
                c.value
                    .parse::<IpNet>()
                    .map_err(|e| format!("invalid CIDR in allow list '{}': {}", c.value, e))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let cidr_deny = spec
            .cidr
            .value
            .deny
            .iter()
            .map(|c| {
                c.value
                    .parse::<IpNet>()
                    .map_err(|e| format!("invalid CIDR in deny list '{}': {}", c.value, e))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            cidr_allow,
            cidr_deny,
            on_no_peer_addr: if spec.on_no_peer_addr.value == ON_NO_PEER_ADDR_DENY {
                OnNoPeerAddr::Deny
            } else {
                OnNoPeerAddr::Allow
            },
            ip_family_ipv4: spec.ip_family.value.ipv4.value,
            ip_family_ipv6: spec.ip_family.value.ipv6.value,
        })
    }
}

#[derive(Debug, Deserialize, Default, Serialize, Clone, PartialEq)]
pub enum OnNoPeerAddr {
    #[default]
    Allow,
    Deny,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CidrSpec, IpFamilySpec, ON_NO_PEER_ADDR_ALLOW};
    use confval::provenance::Located;

    fn filter(
        allow: Vec<&str>,
        deny: Vec<&str>,
        on_no_peer_addr: &str,
    ) -> NetworkConnectionFilterSpec {
        NetworkConnectionFilterSpec {
            cidr: Located::detached(CidrSpec {
                allow: allow
                    .into_iter()
                    .map(|c| Located::detached(c.to_string()))
                    .collect(),
                deny: deny
                    .into_iter()
                    .map(|c| Located::detached(c.to_string()))
                    .collect(),
            }),
            ip_family: Located::detached(IpFamilySpec {
                ipv4: Located::detached(true),
                ipv6: Located::detached(false),
            }),
            on_no_peer_addr: Located::detached(on_no_peer_addr.to_string()),
        }
    }

    #[test]
    fn valid_cidr_parsed() {
        // Arrange
        let spec = filter(
            vec!["10.0.0.0/8"],
            vec!["192.168.0.0/16"],
            ON_NO_PEER_ADDR_ALLOW,
        );

        // Act
        let config = NetworkConnectionFilterConfig::try_from(&spec).unwrap();

        // Assert
        assert_eq!(config.cidr_allow.len(), 1);
        assert_eq!(config.cidr_allow[0], "10.0.0.0/8".parse().unwrap());
        assert_eq!(config.cidr_deny.len(), 1);
        assert_eq!(config.cidr_deny[0], "192.168.0.0/16".parse().unwrap());
        assert!(config.ip_family_ipv4);
        assert!(!config.ip_family_ipv6);
    }

    #[test]
    fn invalid_cidr_fails() {
        // Arrange
        let spec = filter(vec!["not-a-cidr"], vec![], ON_NO_PEER_ADDR_ALLOW);

        // Act
        let result = NetworkConnectionFilterConfig::try_from(&spec);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn on_no_peer_addr_allow() {
        // Arrange
        let spec = filter(vec![], vec![], ON_NO_PEER_ADDR_ALLOW);

        // Act
        let config = NetworkConnectionFilterConfig::try_from(&spec).unwrap();

        // Assert
        assert!(matches!(config.on_no_peer_addr, OnNoPeerAddr::Allow));
    }

    #[test]
    fn on_no_peer_addr_deny() {
        // Arrange
        let spec = filter(vec![], vec![], ON_NO_PEER_ADDR_DENY);

        // Act
        let config = NetworkConnectionFilterConfig::try_from(&spec).unwrap();

        // Assert
        assert!(matches!(config.on_no_peer_addr, OnNoPeerAddr::Deny));
    }
}
