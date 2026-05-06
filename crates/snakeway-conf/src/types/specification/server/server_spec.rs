use crate::types::{ObservabilitySpec, Origin, TlsAutomationSpec};
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

    pub ca_file: Option<PathBuf>,

    pub tls_automation: Option<TlsAutomationSpec>,

    pub observability: Option<ObservabilitySpec>,

    #[serde(default = "default_dns_refresh_interval_seconds")]
    pub dns_refresh_interval_seconds: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub shutdown: Option<ShutdownSpec>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub upgrade: Option<UpgradeSpec>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub performance: Option<PerformanceSpec>,

    /// Local IP addresses used as the source for outbound upstream connections.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_source_addresses: Option<UpstreamSourceAddressesSpec>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ShutdownSpec {
    /// How long active connections are allowed to finish after a shutdown signal.
    #[serde(default = "default_shutdown_drain_seconds")]
    pub drain_seconds: Option<u64>,

    /// Hard ceiling on total shutdown time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_timeout_seconds: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct UpgradeSpec {
    /// Path to the Unix domain socket used for zero-drop upgrades.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sock: Option<String>,

    /// Maximum number of retries when connecting/accepting on the upgrade socket.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PerformanceSpec {
    #[serde(default = "default_work_stealing")]
    pub work_stealing: bool,

    /// Number of idle upstream connections kept warm per worker thread.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_connection_pool_size: Option<usize>,

    /// Number of parallel accept tasks per listener.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_accepts_per_listener: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct UpstreamSourceAddressesSpec {
    #[serde(default)]
    pub ipv4: Vec<String>,
    #[serde(default)]
    pub ipv6: Vec<String>,
}

fn default_work_stealing() -> bool {
    true
}

fn default_dns_refresh_interval_seconds() -> u64 {
    30
}

fn default_shutdown_drain_seconds() -> Option<u64> {
    Some(10)
}

impl Default for ShutdownSpec {
    fn default() -> Self {
        Self {
            drain_seconds: default_shutdown_drain_seconds(),
            force_timeout_seconds: None,
        }
    }
}

impl Default for PerformanceSpec {
    fn default() -> Self {
        Self {
            work_stealing: true,
            upstream_connection_pool_size: None,
            parallel_accepts_per_listener: None,
        }
    }
}

impl Default for ServerSpec {
    fn default() -> Self {
        Self {
            origin: Default::default(),
            version: 1,
            threads: None,
            pid_file: None,
            ca_file: None,
            tls_automation: None,
            observability: None,
            dns_refresh_interval_seconds: 30,
            shutdown: None,
            upgrade: None,
            performance: None,
            upstream_source_addresses: None,
        }
    }
}
