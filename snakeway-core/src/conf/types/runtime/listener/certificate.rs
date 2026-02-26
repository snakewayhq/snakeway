use crate::conf::types::{CertificateChallengeSpec, CertificateSpec};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Paths are validated and resolved during config validation.
/// Runtime code assumes these values are valid.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum CertificateConfig {
    Static {
        cert: PathBuf,
        key: PathBuf,
    },
    Acme {
        domains: Vec<String>,
        challenge: CertificateChallengeConfig,
    },
}

impl TryFrom<CertificateSpec> for CertificateConfig {
    type Error = String;

    fn try_from(spec: CertificateSpec) -> Result<Self, Self::Error> {
        match spec {
            CertificateSpec::Static { cert, key } => Ok(CertificateConfig::Static { cert, key }),

            CertificateSpec::Acme { domains, challenge } => {
                let mut canonicalize_domains: Vec<String> = domains
                    .iter()
                    .map(|d| d.trim().to_ascii_lowercase())
                    .filter(|d| !d.is_empty())
                    .collect();

                canonicalize_domains.sort();
                canonicalize_domains.dedup();

                Ok(CertificateConfig::Acme {
                    domains: canonicalize_domains,
                    challenge: challenge.into(),
                })
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum CertificateChallengeConfig {
    Http01,
}

impl From<CertificateChallengeSpec> for CertificateChallengeConfig {
    fn from(config: CertificateChallengeSpec) -> Self {
        match config {
            CertificateChallengeSpec::Http01 => CertificateChallengeConfig::Http01,
        }
    }
}
