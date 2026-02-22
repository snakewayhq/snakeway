use crate::conf::types::{CertStoreSpec, ServerSpec, TlsServerSpec};
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

    pub tls: Option<TlsServerConfig>,
}

impl From<ServerSpec> for ServerConfig {
    fn from(spec: ServerSpec) -> Self {
        Self {
            version: spec.version,
            threads: spec.threads,
            pid_file: spec.pid_file.unwrap_or_default(),
            ca_file: spec.ca_file.unwrap_or_default(),
            work_stealing: spec.work_stealing,
            tls: spec.tls.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TlsServerConfig {
    pub cert_store: CertStoreConfig,
    pub path: Option<PathBuf>,
}

impl From<TlsServerSpec> for TlsServerConfig {
    fn from(spec: TlsServerSpec) -> Self {
        Self {
            cert_store: spec.cert_store.into(),
            path: spec.path,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum CertStoreConfig {
    Filesystem(PathBuf),
    Memory,
}

impl From<CertStoreSpec> for CertStoreConfig {
    fn from(spec: CertStoreSpec) -> Self {
        match spec {
            CertStoreSpec::Filesystem(path) => Self::Filesystem(path),
            CertStoreSpec::Memory => Self::Memory,
        }
    }
}
