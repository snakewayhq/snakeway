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

    pub observability: Option<ObservabilityConfig>,
}

//-----------------------------------------------------------------------------
// TLS Automation
//-----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TlsAutomationConfig {
    pub acme: AcmeServerConfig,
    pub cert_store: CertStoreConfig,
    pub renew_within_days: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum CertStoreConfig {
    Filesystem { cert_dir: PathBuf },
    Memory,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AcmeServerConfig {
    pub directory_url: String,
    pub data_dir: PathBuf,
    pub contact_email: Vec<String>,
    pub ca_file: Option<PathBuf>,
}

//-----------------------------------------------------------------------------
// Observability
//-----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub struct ObservabilityConfig {
    pub otel: Option<OtelConfig>,
}

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub struct OtelConfig {
    pub enable: bool,
    pub endpoint: String,
    pub service_name: String,
    pub sampling: SamplingTypeConfig,
}

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SamplingTypeConfig {
    #[default]
    ParentBased,
}
