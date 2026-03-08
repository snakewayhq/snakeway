use crate::conf::types::Origin;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ServerSpec {
    #[serde(skip)]
    pub(crate) origin: Origin,

    /// Configuration schema version
    pub(crate) version: u32,

    /// Optional number of worker threads - default is decided by Pingora.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) threads: Option<usize>,

    /// Optional pid file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) pid_file: Option<PathBuf>,

    #[serde(default = "default_work_stealing")]
    pub(crate) work_stealing: bool,

    pub(crate) ca_file: Option<PathBuf>,

    pub(crate) tls_automation: Option<TlsAutomationSpec>,

    pub(crate) observability: Option<ObservabilitySpec>,
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
pub(crate) struct TlsAutomationSpec {
    pub(crate) acme: AcmeServerSpec,
    pub(crate) cert_store: CertStoreSpec,
    #[serde(default = "default_renew_within_days")]
    pub(crate) renew_within_days: u64,
}

fn default_renew_within_days() -> u64 {
    30
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(crate) enum CertStoreSpec {
    Filesystem {
        cert_dir: PathBuf,
    },
    #[default]
    Memory,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) struct AcmeServerSpec {
    pub(crate) directory_url: String,
    pub(crate) data_dir: PathBuf,
    pub(crate) contact_email: Vec<String>,
    pub(crate) ca_file: Option<PathBuf>,
}

//-----------------------------------------------------------------------------
// Observability
//-----------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default, Serialize)]
pub(crate) struct ObservabilitySpec {
    pub(crate) otel: Option<OtelSpec>,
}

#[derive(Debug, Deserialize, Default, Serialize)]
pub(crate) struct OtelSpec {
    pub(crate) enable: bool,
    pub(crate) endpoint: String,
    pub(crate) service_name: String,
    pub(crate) sampling: SamplingTypeSpec,
}

#[derive(Debug, Deserialize, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SamplingTypeSpec {
    #[default]
    ParentBased,
}
