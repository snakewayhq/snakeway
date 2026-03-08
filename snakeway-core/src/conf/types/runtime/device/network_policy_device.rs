use crate::conf::types::{NetworkPolicyDeviceSpec, OnInvalidForwardedSpec};
use crate::conf::validation::ConfigError;
use ipnet::IpNet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct NetworkPolicyDeviceConfig {
    pub(crate) enable: bool,
    pub(crate) cidr_allow: Vec<IpNet>,
    pub(crate) forwarding: ForwardingConfig,
}

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
        })
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct ForwardingConfig {
    pub(crate) allow: bool,
    pub(crate) on_invalid: OnInvalidForwardedConfig,
}

impl From<OnInvalidForwardedSpec> for OnInvalidForwardedConfig {
    fn from(spec: OnInvalidForwardedSpec) -> Self {
        match spec {
            OnInvalidForwardedSpec::Deny => OnInvalidForwardedConfig::Deny,
            OnInvalidForwardedSpec::Ignore => OnInvalidForwardedConfig::Ignore,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OnInvalidForwardedConfig {
    Deny,
    #[default]
    Ignore,
}
