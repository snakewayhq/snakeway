use crate::types::{
    AcmeServerSpec, CertStoreSpec, ObservabilitySpec, OtelSpec, PerformanceSpec, ServerSpec,
    ShutdownSpec, TlsAutomationSpec, UpgradeSpec, UpstreamSourceAddressesSpec,
};
use o2o::o2o;
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

#[derive(o2o, Debug, Clone, Deserialize, Serialize)]
#[from_owned(ShutdownSpec)]
pub struct ShutdownConfig {
    /// How long active connections are allowed to finish after a shutdown signal.
    #[map(~.map(|v| v as u64))]
    pub drain_seconds: Option<u64>,
    /// Hard ceiling on total shutdown time.
    #[map(~.map(|v| v as u64))]
    pub force_timeout_seconds: Option<u64>,
}

//-----------------------------------------------------------------------------
// Upgrade
//-----------------------------------------------------------------------------

#[derive(o2o, Debug, Clone, Deserialize, Serialize)]
#[from_owned(UpgradeSpec)]
pub struct UpgradeConfig {
    /// Path to the Unix domain socket used for zero-drop upgrades (FD transfer).
    pub sock: Option<String>,
    /// Maximum retries when connecting/accepting on the upgrade socket.
    #[map(~.map(|v| v as usize))]
    pub max_retries: Option<usize>,
}

//-----------------------------------------------------------------------------
// Performance
//-----------------------------------------------------------------------------

#[derive(o2o, Debug, Clone, Deserialize, Serialize)]
#[from_owned(PerformanceSpec)]
pub struct PerformanceConfig {
    /// Enable work stealing between threads.
    pub work_stealing: bool,
    /// Number of idle upstream connections kept warm per worker thread.
    #[map(~.map(|v| v as usize))]
    pub upstream_connection_pool_size: Option<usize>,
    /// Number of parallel accept tasks per listener.
    #[map(~.map(|v| v as usize))]
    pub parallel_accepts_per_listener: Option<usize>,
}

//-----------------------------------------------------------------------------
// Upstream Source Addresses
//-----------------------------------------------------------------------------

#[derive(o2o, Debug, Clone, Deserialize, Serialize)]
#[from_owned(UpstreamSourceAddressesSpec)]
pub struct UpstreamSourceAddressesConfig {
    pub ipv4: Vec<String>,
    pub ipv6: Vec<String>,
}

//-----------------------------------------------------------------------------
// TLS Automation
//-----------------------------------------------------------------------------

#[derive(o2o, Debug, Clone, Deserialize, Serialize)]
#[from_owned(TlsAutomationSpec)]
pub struct TlsAutomationConfig {
    #[map(~.into())]
    pub acme: AcmeServerConfig,
    #[map(~.into())]
    pub cert_store: CertStoreConfig,
    #[map(~ as u64)]
    pub renew_within_days: u64,
}

#[derive(o2o, Debug, Clone, Deserialize, Serialize)]
#[from_owned(CertStoreSpec)]
pub enum CertStoreConfig {
    Filesystem { cert_dir: PathBuf },
    Memory,
}

#[derive(o2o, Debug, Clone, Deserialize, Serialize)]
#[from_owned(AcmeServerSpec)]
pub struct AcmeServerConfig {
    pub directory_url: String,
    pub data_dir: PathBuf,
    pub contact_email: Vec<String>,
    pub ca_file: Option<PathBuf>,
}

//-----------------------------------------------------------------------------
// Observability
//-----------------------------------------------------------------------------

#[derive(o2o, Debug, Clone, Deserialize, Default, Serialize)]
#[from_owned(ObservabilitySpec)]
pub struct ObservabilityConfig {
    #[map(~.map(Into::into))]
    pub otel: Option<OtelConfig>,
}

#[derive(o2o, Debug, Clone, Deserialize, Default, Serialize)]
#[from_owned(OtelSpec)]
pub struct OtelConfig {
    pub enable: bool,
    pub endpoint: String,
    pub service_name: String,
    pub sampling_ratio: f64,
}

impl TryFrom<ServerSpec> for ServerConfig {
    type Error = String;
    fn try_from(spec: ServerSpec) -> Result<Self, Self::Error> {
        Ok(Self {
            version: spec.version as u32,
            threads: spec.threads.map(|v| v as usize),
            pid_file: spec.pid_file.unwrap_or_default(),
            ca_file: spec
                .ca_file
                .map(|p| p.into_os_string().into_string())
                .transpose()
                .map_err(|_| {
                    "invalid ca_file path. this likely a bug as it should have been caught by validation".to_string()
                })?,
            tls_automation: spec.tls_automation.map(Into::into),
            observability: spec.observability.map(Into::into),
            dns_refresh_interval_seconds: spec.dns_refresh_interval_seconds as u64,
            shutdown: ShutdownConfig::from(spec.shutdown.unwrap_or_default()),
            upgrade: UpgradeConfig::from(spec.upgrade.unwrap_or_default()),
            performance: PerformanceConfig::from(spec.performance.unwrap_or_default()),
            upstream_source_addresses: spec.upstream_source_addresses.map(Into::into),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn server_config_from_valid_spec() {
        // Arrange
        let spec = ServerSpec::default();

        // Act
        let config = ServerConfig::try_from(spec).unwrap();

        // Assert
        assert_eq!(config.version, 1);
        assert!(config.threads.is_none());
        assert!(config.performance.work_stealing);
        assert!(config.ca_file.is_none());
        assert!(config.tls_automation.is_none());
        assert!(config.observability.is_none());
        assert_eq!(config.dns_refresh_interval_seconds, 30);
        assert_eq!(config.shutdown.drain_seconds, Some(10));
        assert!(config.shutdown.force_timeout_seconds.is_none());
        assert!(config.upgrade.sock.is_none());
        assert!(config.upgrade.max_retries.is_none());
        assert!(config.performance.upstream_connection_pool_size.is_none());
        assert!(config.performance.parallel_accepts_per_listener.is_none());
    }

    #[test]
    fn dns_refresh_interval_explicit_value() {
        // Arrange
        let spec = ServerSpec {
            dns_refresh_interval_seconds: 60,
            ..Default::default()
        };

        // Act
        let config = ServerConfig::try_from(spec).unwrap();

        // Assert
        assert_eq!(config.dns_refresh_interval_seconds, 60);
    }

    #[test]
    fn shutdown_defaults_when_absent() {
        // Arrange
        let spec = ServerSpec::default();

        // Act
        let config = ServerConfig::try_from(spec).unwrap();

        // Assert
        assert_eq!(config.shutdown.drain_seconds, Some(10));
        assert!(config.shutdown.force_timeout_seconds.is_none());
    }

    #[test]
    fn shutdown_from_explicit_spec() {
        // Arrange
        let spec = ServerSpec {
            shutdown: Some(ShutdownSpec {
                drain_seconds: Some(30),
                force_timeout_seconds: Some(60),
            }),
            ..Default::default()
        };

        // Act
        let config = ServerConfig::try_from(spec).unwrap();

        // Assert
        assert_eq!(config.shutdown.drain_seconds, Some(30));
        assert_eq!(config.shutdown.force_timeout_seconds, Some(60));
    }

    #[test]
    fn performance_defaults_when_absent() {
        // Arrange
        let spec = ServerSpec::default();

        // Act
        let config = ServerConfig::try_from(spec).unwrap();

        // Assert
        assert!(config.performance.work_stealing);
        assert!(config.performance.upstream_connection_pool_size.is_none());
        assert!(config.performance.parallel_accepts_per_listener.is_none());
    }

    #[test]
    fn performance_from_explicit_spec() {
        // Arrange
        let spec = ServerSpec {
            performance: Some(PerformanceSpec {
                work_stealing: false,
                upstream_connection_pool_size: Some(256),
                parallel_accepts_per_listener: Some(4),
            }),
            ..Default::default()
        };

        // Act
        let config = ServerConfig::try_from(spec).unwrap();

        // Assert
        assert!(!config.performance.work_stealing);
        assert_eq!(config.performance.upstream_connection_pool_size, Some(256));
        assert_eq!(config.performance.parallel_accepts_per_listener, Some(4));
    }

    #[test]
    fn tls_automation_from_spec() {
        // Arrange
        let spec = TlsAutomationSpec {
            acme: AcmeServerSpec {
                directory_url: "https://acme.example.com/dir".to_string(),
                data_dir: PathBuf::from("/tmp/acme"),
                contact_email: vec!["admin@example.com".to_string()],
                ca_file: None,
            },
            cert_store: CertStoreSpec::Memory,
            renew_within_days: 30,
        };

        // Act
        let config: TlsAutomationConfig = spec.into();

        // Assert
        assert_eq!(config.renew_within_days, 30);
        assert!(matches!(config.cert_store, CertStoreConfig::Memory));
        assert_eq!(config.acme.directory_url, "https://acme.example.com/dir");
    }

    #[test]
    fn cert_store_filesystem_from_spec() {
        // Arrange
        let spec = CertStoreSpec::Filesystem {
            cert_dir: PathBuf::from("/etc/certs"),
        };

        // Act
        let config: CertStoreConfig = spec.into();

        // Assert
        assert!(matches!(
            config,
            CertStoreConfig::Filesystem { cert_dir } if cert_dir == std::path::Path::new("/etc/certs")
        ));
    }

    #[test]
    fn cert_store_memory_from_spec() {
        // Arrange
        let spec = CertStoreSpec::Memory;

        // Act
        let config: CertStoreConfig = spec.into();

        // Assert
        assert!(matches!(config, CertStoreConfig::Memory));
    }
}
