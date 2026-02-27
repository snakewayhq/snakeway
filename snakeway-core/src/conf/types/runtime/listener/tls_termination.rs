use crate::conf::types::{AcmeChallengeSpec, TlsTerminationSpec};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Paths are validated and resolved during config validation.
/// Runtime code assumes these values are valid.
#[derive(Debug, Clone, Deserialize, Serialize)]
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

impl TryFrom<TlsTerminationSpec> for TlsTerminationConfig {
    type Error = String;

    fn try_from(spec: TlsTerminationSpec) -> Result<Self, Self::Error> {
        match spec {
            TlsTerminationSpec::Manual { cert, key } => {
                Ok(TlsTerminationConfig::Manual { cert, key })
            }

            TlsTerminationSpec::Acme { domains, challenge } => {
                let mut canonicalize_domains: Vec<String> = domains
                    .iter()
                    .map(|d| d.trim().to_ascii_lowercase())
                    .filter(|d| !d.is_empty())
                    .collect();

                canonicalize_domains.sort();
                canonicalize_domains.dedup();

                Ok(TlsTerminationConfig::Acme {
                    domains: canonicalize_domains,
                    challenge: challenge.into(),
                })
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum AcmeChallengeConfig {
    Http01,
}

impl From<AcmeChallengeSpec> for AcmeChallengeConfig {
    fn from(config: AcmeChallengeSpec) -> Self {
        match config {
            AcmeChallengeSpec::Http01 => AcmeChallengeConfig::Http01,
        }
    }
}
