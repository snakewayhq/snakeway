use crate::conf::types::Origin;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerSpec {
    #[serde(skip)]
    pub origin: Origin,

    /// Configuration schema version
    pub version: u32,

    /// Optional number of worker threads - default is decided by Pingora.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads: Option<usize>,

    /// Optional pid file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid_file: Option<PathBuf>,

    #[serde(default = "default_work_stealing")]
    pub work_stealing: bool,

    pub ca_file: Option<PathBuf>,

    pub tls_automation: Option<TlsAutomationSpec>,

    pub observability: Option<ObservabilitySpec>,
}

fn default_work_stealing() -> bool {
    true
}

impl Default for ServerSpec {
    fn default() -> Self {
        Self {
            origin: Default::default(),
            version: 1,
            threads: None,
            pid_file: None,
            work_stealing: true,
            ca_file: None,
            tls_automation: None,
            observability: None,
        }
    }
}

//-----------------------------------------------------------------------------
// TLS Automation
//-----------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct TlsAutomationSpec {
    pub acme: AcmeServerSpec,
    pub cert_store: CertStoreSpec,
    #[serde(default = "default_renew_within_days")]
    pub renew_within_days: u64,
}

fn default_renew_within_days() -> u64 {
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

//-----------------------------------------------------------------------------
// Observability
//-----------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct ObservabilitySpec {
    pub otel: Option<OtelSpec>,
}

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct OtelSpec {
    pub enable: bool,
    pub endpoint: String,
    pub service_name: String,
    pub sampling: SamplingTypeSpec,
}

#[derive(Debug, Deserialize, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SamplingTypeSpec {
    #[default]
    ParentBased,
}
