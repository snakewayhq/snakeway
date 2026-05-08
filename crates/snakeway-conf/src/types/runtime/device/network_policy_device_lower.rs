use crate::types::{NetworkPolicyDeviceSpec, OnInvalidForwardedSpec};
use crate::validation::ConfigError;
use ipnet::IpNet;

use super::{ForwardingConfig, NetworkPolicyDeviceConfig, OnInvalidForwardedConfig};

impl TryFrom<NetworkPolicyDeviceSpec> for NetworkPolicyDeviceConfig {
    type Error = ConfigError;

    fn try_from(spec: NetworkPolicyDeviceSpec) -> Result<Self, Self::Error> {
        let cidr_allow = spec
            .cidr_allow
            .iter()
            .map(|c| {
                c.parse::<IpNet>()
                    .map_err(|e| ConfigError::InvalidUpstream {
                        message: format!("invalid network policy CIDR '{}': {}", c, e),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            enable: spec.enable,
            cidr_allow,
            forwarding: ForwardingConfig {
                allow: spec.forwarding.allow,
                on_invalid: spec.forwarding.on_invalid.into(),
            },
            paths: spec.paths.into_iter().collect(),
        })
    }
}

impl From<OnInvalidForwardedSpec> for OnInvalidForwardedConfig {
    fn from(spec: OnInvalidForwardedSpec) -> Self {
        match spec {
            OnInvalidForwardedSpec::Deny => OnInvalidForwardedConfig::Deny,
            OnInvalidForwardedSpec::Ignore => OnInvalidForwardedConfig::Ignore,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ForwardingSpec, HclOrigin};
    use std::path::PathBuf;

    fn test_origin() -> HclOrigin {
        HclOrigin {
            file: PathBuf::from("test.hcl"),
            section: "device.network_policy".to_string(),
            index: None,
        }
    }

    #[test]
    fn valid_cidr_parsed() {
        // Arrange
        let spec = NetworkPolicyDeviceSpec {
            origin: test_origin(),
            enable: true,
            cidr_allow: vec!["10.0.0.0/8".to_string()],
            forwarding: ForwardingSpec::default(),
            paths: vec![],
        };

        // Act
        let config = NetworkPolicyDeviceConfig::try_from(spec).unwrap();

        // Assert
        assert_eq!(config.cidr_allow.len(), 1);
        assert_eq!(config.cidr_allow[0], "10.0.0.0/8".parse::<IpNet>().unwrap());
    }

    #[test]
    fn invalid_cidr_fails() {
        // Arrange
        let spec = NetworkPolicyDeviceSpec {
            origin: test_origin(),
            enable: true,
            cidr_allow: vec!["not-a-cidr".to_string()],
            forwarding: ForwardingSpec::default(),
            paths: vec![],
        };

        // Act
        let result = NetworkPolicyDeviceConfig::try_from(spec);

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::InvalidUpstream { .. }
        ));
    }

    #[test]
    fn on_invalid_forwarded_deny() {
        // Arrange
        let spec = OnInvalidForwardedSpec::Deny;

        // Act
        let config: OnInvalidForwardedConfig = spec.into();

        // Assert
        assert!(matches!(config, OnInvalidForwardedConfig::Deny));
    }

    #[test]
    fn on_invalid_forwarded_ignore() {
        // Arrange
        let spec = OnInvalidForwardedSpec::Ignore;

        // Act
        let config: OnInvalidForwardedConfig = spec.into();

        // Assert
        assert!(matches!(config, OnInvalidForwardedConfig::Ignore));
    }
}
