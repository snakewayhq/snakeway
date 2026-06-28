use crate::types::{NetworkConnectionFilterSpec, ON_NO_PEER_ADDR_DENY};
use crate::validation::validator::parse_cidr_list;
use confval::prelude::{Lower, Report};
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

impl Lower<NetworkConnectionFilterSpec> for NetworkConnectionFilterConfig {
    fn lower(spec: &NetworkConnectionFilterSpec, report: &mut Report) -> Option<Self> {
        let cidr_allow = parse_cidr_list(&spec.cidr.value.allow, "allow list", report);
        let cidr_deny = parse_cidr_list(&spec.cidr.value.deny, "deny list", report);

        Some(Self {
            cidr_allow: cidr_allow?,
            cidr_deny: cidr_deny?,
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
    use confval::prelude::Located;

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
        let mut report = Report::new();

        // Act
        let config = NetworkConnectionFilterConfig::lower(&spec, &mut report).unwrap();

        // Assert
        assert!(!report.has_errors());
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
        let mut report = Report::new();

        // Act
        let result = NetworkConnectionFilterConfig::lower(&spec, &mut report);

        // Assert
        assert!(result.is_none());
        assert!(report.has_errors());
        assert!(report.issues().iter().any(|i| {
            i.message
                .contains("invalid CIDR in allow list 'not-a-cidr'")
        }));
    }

    #[test]
    fn every_invalid_cidr_reported() {
        // Arrange
        let spec = filter(vec!["bad-one"], vec!["bad-two"], ON_NO_PEER_ADDR_ALLOW);
        let mut report = Report::new();

        // Act
        let result = NetworkConnectionFilterConfig::lower(&spec, &mut report);

        // Assert
        assert!(result.is_none());
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message.contains("allow list 'bad-one'"))
        );
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message.contains("deny list 'bad-two'"))
        );
    }

    #[test]
    fn on_no_peer_addr_allow() {
        // Arrange
        let spec = filter(vec![], vec![], ON_NO_PEER_ADDR_ALLOW);
        let mut report = Report::new();

        // Act
        let config = NetworkConnectionFilterConfig::lower(&spec, &mut report).unwrap();

        // Assert
        assert!(matches!(config.on_no_peer_addr, OnNoPeerAddr::Allow));
    }

    #[test]
    fn on_no_peer_addr_deny() {
        // Arrange
        let spec = filter(vec![], vec![], ON_NO_PEER_ADDR_DENY);
        let mut report = Report::new();

        // Act
        let config = NetworkConnectionFilterConfig::lower(&spec, &mut report).unwrap();

        // Assert
        assert!(matches!(config.on_no_peer_addr, OnNoPeerAddr::Deny));
    }
}
