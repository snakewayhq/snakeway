use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TlsSpec {
    pub mode: TlsManagementModeSpec,

    // Static mode
    pub cert: Option<String>,
    pub key: Option<String>,

    // Acme mode
    pub domains: Option<Vec<String>>,
    pub challenge: Option<ChallengeSpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TlsManagementModeSpec {
    #[default]
    Static,
    Acme,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeSpec {
    #[default]
    Http01,
}
