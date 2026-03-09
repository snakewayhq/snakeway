use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum TlsTerminationSpec {
    Manual {
        cert: PathBuf,
        key: PathBuf,
    },
    Acme {
        domains: Vec<String>,
        #[serde(default)]
        challenge: AcmeChallengeSpec,
    },
}

impl Default for TlsTerminationSpec {
    fn default() -> Self {
        TlsTerminationSpec::Manual {
            cert: PathBuf::new(),
            key: PathBuf::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AcmeChallengeSpec {
    #[default]
    Http01,
}
