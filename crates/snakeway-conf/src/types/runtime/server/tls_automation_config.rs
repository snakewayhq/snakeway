use crate::types::CertStoreSpec;
use confval::diagnostic::Report;
use confval::pipeline::Lower;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize, confval::Config)]
#[confval(lower_from = TlsAutomationSpec, validate)]
pub struct TlsAutomationConfig {
    #[confval(nested)]
    pub acme: AcmeServerConfig,
    #[confval(nested)]
    pub cert_store: CertStoreConfig,
    #[confval(lower(from = renew_within_days, with = narrow::i64_to_u64))]
    pub renew_within_days: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, confval::Config)]
#[confval(lower_from = AcmeServerSpec)]
pub struct AcmeServerConfig {
    pub directory_url: String,
    pub data_dir: PathBuf,
    pub contact_email: Vec<String>,
    pub ca_file: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum CertStoreConfig {
    Filesystem { cert_dir: PathBuf },
    Memory,
}

impl Lower<CertStoreSpec> for CertStoreConfig {
    fn lower(spec: &CertStoreSpec, _report: &mut Report) -> Option<Self> {
        Some(match spec {
            CertStoreSpec::Filesystem { cert_dir } => CertStoreConfig::Filesystem {
                cert_dir: cert_dir.value.clone(),
            },
            CertStoreSpec::Memory => CertStoreConfig::Memory,
        })
    }
}
