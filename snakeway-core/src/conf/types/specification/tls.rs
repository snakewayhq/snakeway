use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TlsSpec {
    pub cert: String,
    pub key: String,
    // pub mode: TlsModeSpec,
}

// #[derive(Debug, Clone, Deserialize, Serialize, Default)]
// pub enum TlsModeSpec {
//     #[default]
//     Static,
//     Acme,
// }
//
// #[derive(Debug, Deserialize, Serialize, Default)]
// #[serde(rename_all = "snake_case")]
// pub struct AcmeSpec {
//     pub domains: Vec<String>,
//     pub challenge: String,
// }
//
// #[derive(Debug, Clone, Deserialize, Serialize, Default)]
// pub enum ChallengeSpec {
//     #[default]
//     Http01,
// }
