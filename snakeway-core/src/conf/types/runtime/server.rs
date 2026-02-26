use crate::conf::types::{AcmeServerSpec, CertStoreSpec, CertificatesSpec, ServerSpec};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub version: u32,

    /// Optional number of worker threads - default is decided by Pingora.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads: Option<usize>,

    /// Pid file path.
    /// If empty, Snakeway will not write a pid file.
    pub pid_file: PathBuf,

    /// CA file path.
    /// If set/not empty, Pingora will use this file to verify upstream certificates.
    pub ca_file: String,

    /// Enable work stealing between threads.
    pub work_stealing: bool,

    pub certificates: Option<CertificatesConfig>,
}

impl From<ServerSpec> for ServerConfig {
    fn from(spec: ServerSpec) -> Self {
        Self {
            version: spec.version,
            threads: spec.threads,
            pid_file: spec.pid_file.unwrap_or_default(),
            ca_file: spec.ca_file.unwrap_or_default(),
            work_stealing: spec.work_stealing,
            certificates: spec.certificates.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CertificatesConfig {
    pub acme: AcmeServerConfig,
    pub cert_store: CertStoreConfig,
    pub renew_within_days: u64,
}

impl From<CertificatesSpec> for CertificatesConfig {
    fn from(spec: CertificatesSpec) -> Self {
        Self {
            cert_store: spec.cert_store.into(),
            renew_within_days: spec.renew_within_days,
            acme: spec.acme.into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum CertStoreConfig {
    Filesystem { cert_dir: PathBuf },
    Memory,
}

impl From<CertStoreSpec> for CertStoreConfig {
    fn from(spec: CertStoreSpec) -> Self {
        match spec {
            CertStoreSpec::Filesystem { cert_dir } => Self::Filesystem { cert_dir },
            CertStoreSpec::Memory => Self::Memory,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AcmeServerConfig {
    pub directory_url: String,
    pub data_dir: PathBuf,
    pub contact_email: Vec<String>,
}

impl From<AcmeServerSpec> for AcmeServerConfig {
    fn from(spec: AcmeServerSpec) -> Self {
        Self {
            directory_url: spec.directory_url,
            data_dir: spec.data_dir,
            contact_email: spec.contact_email,
        }
    }
}
