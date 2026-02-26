use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum CertificateSpec {
    Static {
        cert: PathBuf,
        key: PathBuf,
    },
    Acme {
        domains: Vec<String>,
        #[serde(default)]
        challenge: CertificateChallengeSpec,
    },
}

impl Default for CertificateSpec {
    fn default() -> Self {
        CertificateSpec::Static {
            cert: PathBuf::new(),
            key: PathBuf::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CertificateChallengeSpec {
    #[default]
    Http01,
}
