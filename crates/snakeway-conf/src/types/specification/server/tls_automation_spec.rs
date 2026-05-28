use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct TlsAutomationSpec {
    pub acme: AcmeServerSpec,
    pub cert_store: CertStoreSpec,
    #[serde(default = "default_renew_within_days")]
    pub renew_within_days: i64,
}

fn default_renew_within_days() -> i64 {
    30
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CertStoreSpec {
    Filesystem {
        cert_dir: PathBuf,
    },
    #[default]
    Memory,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub struct AcmeServerSpec {
    pub directory_url: String,
    pub data_dir: PathBuf,
    pub contact_email: Vec<String>,
    pub ca_file: Option<PathBuf>,
}
