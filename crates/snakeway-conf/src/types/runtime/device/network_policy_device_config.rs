use crate::types::{NetworkPolicyDeviceSpec, ON_NO_PEER_ADDR_DENY};
use crate::validation::ConfigError;
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OnInvalidForwardedConfig {
    Deny,
    #[default]
    Ignore,
}

impl TryFrom<NetworkPolicyDeviceSpec> for NetworkPolicyDeviceConfig {
    type Error = ConfigError;
    fn try_from(spec: NetworkPolicyDeviceSpec) -> Result<Self, Self::Error> {
        let cidr_allow = spec
            .cidr_allow
            .iter()
            .map(|c| {
                c.value
                    .parse::<IpNet>()
                    .map_err(|e| ConfigError::InvalidUpstream {
                        message: format!("invalid network policy CIDR '{}': {}", c.value, e),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            enable: spec.enable.value,
            cidr_allow,
            forwarding: ForwardingConfig {
                allow: spec.forwarding.value.allow.value,
                on_invalid: if spec.forwarding.value.on_invalid.value == ON_NO_PEER_ADDR_DENY {
                    OnInvalidForwardedConfig::Deny
                } else {
                    OnInvalidForwardedConfig::Ignore
                },
            },
            paths: spec.paths.into_iter().map(|p| p.value).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::provenance::Located;

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
        let config = NetworkPolicyDeviceConfig::try_from(spec(vec!["10.0.0.0/8"])).unwrap();

        // Assert
        assert_eq!(config.cidr_allow.len(), 1);
        assert_eq!(config.cidr_allow[0], "10.0.0.0/8".parse::<IpNet>().unwrap());
    }

    #[test]
    fn invalid_cidr_fails() {
        // Arrange / Act
        let result = NetworkPolicyDeviceConfig::try_from(spec(vec!["not-a-cidr"]));

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::InvalidUpstream { .. }
        ));
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
        let config = NetworkPolicyDeviceConfig::try_from(deny).unwrap();

        // Assert
        assert!(matches!(
            config.forwarding.on_invalid,
            OnInvalidForwardedConfig::Deny
        ));
        assert!(config.forwarding.allow);
    }
}
