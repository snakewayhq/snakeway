use crate::conf::types::{ChallengeSpec, TlsManagementModeSpec, TlsSpec};
use serde::{Deserialize, Serialize};

/// Paths are validated and resolved during config validation.
/// Runtime code assumes these values are valid.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TlsConfig {
    pub static_options: Option<StaticTlsConfig>,
    pub acme_options: Option<AcmeTlsConfig>,
}

impl TlsConfig {
    pub fn is_static(&self) -> bool {
        self.static_options.is_some()
    }

    pub fn is_acme(&self) -> bool {
        self.acme_options.is_some()
    }
}

impl TryFrom<TlsSpec> for TlsConfig {
    type Error = String;

    fn try_from(spec: TlsSpec) -> Result<Self, Self::Error> {
        match spec.mode {
            TlsManagementModeSpec::Static => {
                let cert = spec.cert.ok_or("tls.cert required in static mode")?;
                let key = spec.key.ok_or("tls.key required in static mode")?;

                Ok(Self {
                    static_options: Some(StaticTlsConfig { cert, key }),
                    acme_options: None,
                })
            }

            TlsManagementModeSpec::Acme => {
                let domains = spec.domains.ok_or("tls.domains required in acme mode")?;
                if domains.is_empty() {
                    return Err("tls.domains cannot be empty in acme mode".into());
                }

                Ok(Self {
                    static_options: None,
                    acme_options: Some(AcmeTlsConfig {
                        domains,
                        challenge: spec.challenge.unwrap_or_default().into(),
                    }),
                })
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StaticTlsConfig {
    pub cert: String,
    pub key: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AcmeTlsConfig {
    pub domains: Vec<String>,
    pub challenge: ChallengeConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ChallengeConfig {
    Http01,
}

impl From<ChallengeSpec> for ChallengeConfig {
    fn from(config: ChallengeSpec) -> Self {
        match config {
            ChallengeSpec::Http01 => ChallengeConfig::Http01,
        }
    }
}
