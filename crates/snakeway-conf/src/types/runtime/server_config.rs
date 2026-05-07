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

    pub ca_file: Option<String>,

    pub tls_automation: Option<TlsAutomationConfig>,

    pub observability: Option<ObservabilityConfig>,

    pub dns_refresh_interval_seconds: u64,

    pub shutdown: ShutdownConfig,

    pub upgrade: UpgradeConfig,

    pub performance: PerformanceConfig,

    /// Local IP addresses used as the source for outbound upstream connections.
    pub upstream_source_addresses: Option<UpstreamSourceAddressesConfig>,
}

//-----------------------------------------------------------------------------
// Shutdown
//-----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShutdownConfig {
    /// How long active connections are allowed to finish after a shutdown signal.
    pub drain_seconds: Option<u64>,
    /// Hard ceiling on total shutdown time.
    pub force_timeout_seconds: Option<u64>,
}

//-----------------------------------------------------------------------------
// Upgrade
//-----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpgradeConfig {
    /// Path to the Unix domain socket used for zero-drop upgrades (FD transfer).
    pub sock: Option<String>,
    /// Maximum retries when connecting/accepting on the upgrade socket.
    pub max_retries: Option<usize>,
}

//-----------------------------------------------------------------------------
// Performance
//-----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceConfig {
    /// Enable work stealing between threads.
    pub work_stealing: bool,
    /// Number of idle upstream connections kept warm per worker thread.
    pub upstream_connection_pool_size: Option<usize>,
    /// Number of parallel accept tasks per listener.
    pub parallel_accepts_per_listener: Option<usize>,
}

//-----------------------------------------------------------------------------
// Upstream Source Addresses
//-----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpstreamSourceAddressesConfig {
    pub ipv4: Vec<String>,
    pub ipv6: Vec<String>,
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
    pub sampling_ratio: f64,
}
