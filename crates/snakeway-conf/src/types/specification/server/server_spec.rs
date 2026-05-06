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

    #[serde(default = "default_work_stealing")]
    pub work_stealing: bool,

    pub ca_file: Option<PathBuf>,

    pub tls_automation: Option<TlsAutomationSpec>,

    pub observability: Option<ObservabilitySpec>,

    #[serde(default = "default_dns_refresh_interval_seconds")]
    pub dns_refresh_interval_seconds: u64,

    /// Path to the Unix domain socket used for zero-drop upgrades.
    /// Both old and new processes must agree on this path for FD transfer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upgrade_sock: Option<String>,

    /// Maximum number of retries when connecting/accepting on the upgrade socket.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upgrade_max_retries: Option<usize>,

    /// How long active connections are allowed to finish after a shutdown signal.
    /// Connections that complete within this window are guaranteed not to be dropped.
    #[serde(default = "default_shutdown_drain_seconds")]
    pub shutdown_drain_seconds: Option<u64>,

    /// Hard ceiling on total shutdown time. After this timeout, remaining
    /// connections are forcefully terminated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shutdown_force_timeout_seconds: Option<u64>,
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
            dns_refresh_interval_seconds: 30,
            upgrade_sock: None,
            upgrade_max_retries: None,
            shutdown_drain_seconds: default_shutdown_drain_seconds(),
            shutdown_force_timeout_seconds: None,
        }
    }
}
