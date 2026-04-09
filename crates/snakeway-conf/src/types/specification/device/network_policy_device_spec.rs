use crate::types::Origin;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct NetworkPolicyDeviceSpec {
    #[serde(skip)]
    pub origin: Origin,

    pub enable: bool,
    pub cidr_allow: Vec<String>,
    pub forwarding: ForwardingSpec,

    /// Optional path prefixes this device applies to. Empty means all paths.
    #[serde(default)]
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ForwardingSpec {
    pub allow: bool,
    pub on_invalid: OnInvalidForwardedSpec,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OnInvalidForwardedSpec {
    Deny,
    #[default]
    Ignore,
}
