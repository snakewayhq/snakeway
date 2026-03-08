use crate::conf::types::{
    AcmeServerSpec, CertStoreSpec, ObservabilitySpec, OtelSpec, SamplingTypeSpec, ServerSpec,
    TlsAutomationSpec,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ServerConfig {
    pub(crate) version: u32,

    /// Optional number of worker threads - default is decided by Pingora.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) threads: Option<usize>,

    /// Pid file path.
    /// If empty, Snakeway will not write a pid file.
    pub(crate) pid_file: PathBuf,

    /// Enable work stealing between threads.
    pub(crate) work_stealing: bool,

    pub(crate) ca_file: Option<String>,

    pub(crate) tls_automation: Option<TlsAutomationConfig>,

    pub(crate) observability: Option<ObservabilityConfig>,
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
            observability: spec.observability.map(Into::into),
        })
    }
}

//-----------------------------------------------------------------------------
// TLS Automation
//-----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct TlsAutomationConfig {
    pub(crate) acme: AcmeServerConfig,
    pub(crate) cert_store: CertStoreConfig,
    pub(crate) renew_within_days: u64,
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
pub(crate) enum CertStoreConfig {
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
pub(crate) struct AcmeServerConfig {
    pub(crate) directory_url: String,
    pub(crate) data_dir: PathBuf,
    pub(crate) contact_email: Vec<String>,
    pub(crate) ca_file: Option<PathBuf>,
}

impl From<AcmeServerSpec> for AcmeServerConfig {
    fn from(spec: AcmeServerSpec) -> Self {
        Self {
            directory_url: spec.directory_url,
            data_dir: spec.data_dir,
            contact_email: spec.contact_email,
            ca_file: spec.ca_file,
        }
    }
}

//-----------------------------------------------------------------------------
// Observability
//-----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub(crate) struct ObservabilityConfig {
    pub(crate) otel: Option<OtelConfig>,
}

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub(crate) struct OtelConfig {
    pub(crate) enable: bool,
    pub(crate) endpoint: String,
    pub(crate) service_name: String,
    pub(crate) sampling: SamplingTypeConfig,
}

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SamplingTypeConfig {
    #[default]
    ParentBased,
}

impl From<ObservabilitySpec> for ObservabilityConfig {
    fn from(spec: ObservabilitySpec) -> Self {
        Self {
            otel: spec.otel.map(Into::into),
        }
    }
}

impl From<OtelSpec> for OtelConfig {
    fn from(spec: OtelSpec) -> Self {
        Self {
            enable: spec.enable,
            endpoint: spec.endpoint,
            service_name: spec.service_name,
            sampling: spec.sampling.into(),
        }
    }
}

impl From<SamplingTypeSpec> for SamplingTypeConfig {
    fn from(spec: SamplingTypeSpec) -> Self {
        match spec {
            SamplingTypeSpec::ParentBased => Self::ParentBased,
        }
    }
}
