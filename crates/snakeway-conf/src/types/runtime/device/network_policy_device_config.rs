use crate::types::OnInvalidForwardedSpec;
use ipnet::IpNet;
use o2o::o2o;
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

#[derive(o2o, Debug, Clone, Default, Deserialize, Serialize)]
#[from_owned(OnInvalidForwardedSpec)]
#[serde(rename_all = "lowercase")]
pub enum OnInvalidForwardedConfig {
    Deny,
    #[default]
    Ignore,
}

#[cfg(test)]
mod tests {
    use super::*;

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
