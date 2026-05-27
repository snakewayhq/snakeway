use crate::types::AcmeChallengeSpec;
use o2o::o2o;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Paths are validated and resolved during config validation.
/// Runtime code assumes these values are valid.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum TlsTerminationConfig {
    Manual {
        cert: PathBuf,
        key: PathBuf,
    },
    Acme {
        domains: Vec<String>,
        challenge: AcmeChallengeConfig,
    },
}

#[derive(o2o, Debug, Clone, PartialEq, Deserialize, Serialize)]
#[from_owned(AcmeChallengeSpec)]
pub enum AcmeChallengeConfig {
    Http01,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acme_challenge_maps_http01() {
        // Arrange
        let spec = AcmeChallengeSpec::Http01;

        // Act
        let config: AcmeChallengeConfig = spec.into();

        // Assert
        assert!(matches!(config, AcmeChallengeConfig::Http01));
    }
}
