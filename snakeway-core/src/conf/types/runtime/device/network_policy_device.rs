use crate::conf::types::{NetworkPolicyDeviceSpec, OnInvalidForwardedSpec};
use ipnet::IpNet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct NetworkPolicyDeviceConfig {
    pub cidr_allow: Vec<IpNet>,
    pub forwarding: ForwardingConfig,
}

impl From<NetworkPolicyDeviceSpec> for NetworkPolicyDeviceConfig {
    fn from(spec: NetworkPolicyDeviceSpec) -> Self {
        Self {
            cidr_allow: spec
                .cidr_allow
                .iter()
                .map(|c| c.parse().expect("validated CIDR"))
                .collect(),
            forwarding: ForwardingConfig {
                allow: spec.forwarding.allow,
                on_invalid: spec.forwarding.on_invalid.into(),
            },
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ForwardingConfig {
    pub allow: bool,
    pub on_invalid: OnInvalidForwardedConfig,
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
pub enum OnInvalidForwardedConfig {
    Deny,
    #[default]
    Ignore,
}
