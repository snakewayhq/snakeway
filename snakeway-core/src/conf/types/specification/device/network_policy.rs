use crate::conf::types::Origin;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct NetworkPolicyDeviceSpec {
    #[serde(skip)]
    pub(crate) origin: Origin,

    pub(crate) enable: bool,
    pub(crate) cidr_allow: Vec<String>,
    pub(crate) forwarding: ForwardingSpec,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct ForwardingSpec {
    pub(crate) allow: bool,
    pub(crate) on_invalid: OnInvalidForwardedSpec,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OnInvalidForwardedSpec {
    Deny,
    #[default]
    Ignore,
}
