use crate::conf::types::{AcmeServerSpec, CertStoreSpec, ServerSpec, TlsAutomationSpec};
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

    /// Enable work stealing between threads.
    pub work_stealing: bool,

    pub ca_file: Option<String>,

    pub tls_automation: Option<TlsAutomationConfig>,
}

impl TryFrom<ServerSpec> for ServerConfig {
    type Error = String;
    fn try_from(spec: ServerSpec) -> Result<Self, Self::Error> {
        Ok(Self {
            version: spec.version,
            threads: spec.threads,
            pid_file: spec.pid_file.unwrap_or_default(),
            work_stealing: spec.work_stealing,
            ca_file: spec
                .ca_file
                .map(|p| p.into_os_string().into_string())
                .transpose()
                .map_err(|_| {
                    "invalid ca_file path. this likely a bug as it should have been caught by validation".to_string()
                })?,
            tls_automation: spec.tls_automation.map(Into::into),
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TlsAutomationConfig {
    pub acme: AcmeServerConfig,
    pub cert_store: CertStoreConfig,
    pub renew_within_days: u64,
}

impl From<TlsAutomationSpec> for TlsAutomationConfig {
    fn from(spec: TlsAutomationSpec) -> Self {
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
