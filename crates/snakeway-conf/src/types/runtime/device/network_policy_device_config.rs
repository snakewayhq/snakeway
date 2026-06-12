use crate::types::{NetworkPolicyDeviceSpec, ON_NO_PEER_ADDR_DENY};
use confval::provenance::{Lower, Report};
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

impl Lower<NetworkPolicyDeviceSpec> for NetworkPolicyDeviceConfig {
    fn lower(spec: &NetworkPolicyDeviceSpec, report: &mut Report) -> Option<Self> {
        let mut cidr_allow = Vec::with_capacity(spec.cidr_allow.len());
        let mut ok = true;
        for c in &spec.cidr_allow {
            match c.value.parse::<IpNet>() {
                Ok(net) => cidr_allow.push(net),
                Err(e) => {
                    report
                        .error(format!("invalid network policy CIDR '{}': {}", c.value, e))
                        .at(c.span)
                        .emit();
                    ok = false;
                }
            }
        }
        if !ok {
            return None;
        }

        Some(Self {
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
            paths: spec.paths.iter().map(|p| p.value.clone()).collect(),
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
                .contains("invalid network policy CIDR 'not-a-cidr'")
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
