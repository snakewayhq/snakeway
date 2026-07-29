use crate::types::{NetworkPolicyDeviceSpec, OnInvalidForwardedConfig};
use crate::validation::validator::parse_cidr_list;
use confval::prelude::{Lower, Report, Validate, ValidateNested, narrow};
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct NetworkPolicyDeviceConfig {
    pub enable: bool,
    pub cidr_allow: Vec<IpNet>,
    pub forwarding: ForwardingConfig,
    pub paths: SmallVec<[String; 4]>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ForwardingConfig {
    pub allow: bool,
    pub on_invalid: OnInvalidForwardedConfig,
}

impl Lower<NetworkPolicyDeviceSpec> for NetworkPolicyDeviceConfig
where
    NetworkPolicyDeviceSpec: Validate + ValidateNested,
{
    fn lower(spec: &NetworkPolicyDeviceSpec, report: &mut Report) -> Option<Self> {
        let cidr_allow = parse_cidr_list(&spec.cidr_allow, "network policy allow list", report)?;
        let on_invalid =
            narrow::keyword::<OnInvalidForwardedConfig>(&spec.forwarding.value.on_invalid, report)?;

        Some(Self {
            enable: spec.enable.value,
            cidr_allow,
            forwarding: ForwardingConfig {
                allow: spec.forwarding.value.allow.value,
                on_invalid,
            },
            paths: spec.paths.iter().map(|p| p.value.clone()).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::prelude::Located;

    fn spec(cidrs: Vec<&str>) -> NetworkPolicyDeviceSpec {
        NetworkPolicyDeviceSpec {
            enable: Located::detached(true),
            cidr_allow: cidrs
                .into_iter()
                .map(|c| Located::detached(c.to_string()))
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn valid_cidr_parsed() {
        // Arrange / Act
        let config =
            NetworkPolicyDeviceConfig::lower(&spec(vec!["10.0.0.0/8"]), &mut Report::new())
                .unwrap();

        // Assert
        assert_eq!(config.cidr_allow.len(), 1);
        assert_eq!(config.cidr_allow[0], "10.0.0.0/8".parse::<IpNet>().unwrap());
    }

    #[test]
    fn invalid_cidr_fails() {
        // Arrange
        let mut report = Report::new();

        // Act
        let result = NetworkPolicyDeviceConfig::lower(&spec(vec!["not-a-cidr"]), &mut report);

        // Assert
        assert!(result.is_none());
        assert!(report.issues().iter().any(|i| {
            i.message
                .contains("invalid CIDR in network policy allow list 'not-a-cidr'")
        }));
    }

    #[test]
    fn on_invalid_forwarded_keywords() {
        // Arrange
        let mut deny = spec(vec!["10.0.0.0/8"]);
        deny.forwarding = Located::detached(crate::types::ForwardingSpec {
            allow: Located::detached(true),
            on_invalid: Located::detached("deny".to_string()),
        });

        // Act
        let config = NetworkPolicyDeviceConfig::lower(&deny, &mut Report::new()).unwrap();

        // Assert
        assert!(matches!(
            config.forwarding.on_invalid,
            OnInvalidForwardedConfig::Deny
        ));
        assert!(config.forwarding.allow);
    }
}
