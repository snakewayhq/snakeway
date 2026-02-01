use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct NetworkPolicyDeviceSpec {
    pub cidr_allow: Vec<String>,
    pub forwarding: ForwardingSpec,
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
