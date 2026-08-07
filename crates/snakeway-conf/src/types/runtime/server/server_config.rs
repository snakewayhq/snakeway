use crate::types::{
    ObservabilityConfig, PerformanceConfig, ServerSpec, ShutdownConfig, TlsAutomationConfig,
    UpgradeConfig, UpstreamSettingsConfig, WasmConfig,
};
use confval::prelude::narrow;
use confval::prelude::{Located, Report};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize, confval::Config)]
#[confval(lower_from = ServerSpec)]
pub struct ServerConfig {
    #[confval(lower(from = version, with = narrow::i64_to_u32))]
    pub version: u32,

    /// Number of worker threads. When unset, Pingora chooses the value.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(lower(from = threads, with = narrow::opt_i64_to_usize))]
    pub threads: Option<usize>,

    /// Pid file path.
    /// If empty, Snakeway will not write a pid file.
    #[confval(lower(from = pid_file, with = pid_file_or_default))]
    pub pid_file: PathBuf,

    #[confval(lower(from = ca_file, with = ca_file_to_string))]
    pub ca_file: Option<String>,

    #[confval(nested)]
    pub tls_automation: Option<TlsAutomationConfig>,

    #[confval(nested)]
    pub observability: Option<ObservabilityConfig>,

    #[confval(lower(from = dns_refresh_interval_seconds, with = narrow::i64_to_u64))]
    pub dns_refresh_interval_seconds: u64,

    #[confval(nested, default)]
    pub shutdown: ShutdownConfig,

    #[confval(nested, default)]
    pub upgrade: UpgradeConfig,

    #[confval(nested, default)]
    pub performance: PerformanceConfig,

    #[confval(nested, default)]
    pub upstream: UpstreamSettingsConfig,

    #[confval(nested, default)]
    pub wasm: WasmConfig,
}

fn pid_file_or_default(value: &Option<Located<PathBuf>>, _report: &mut Report) -> Option<PathBuf> {
    Some(value.as_ref().map(|p| p.value.clone()).unwrap_or_default())
}

fn ca_file_to_string(
    value: &Option<Located<PathBuf>>,
    report: &mut Report,
) -> Option<Option<String>> {
    match value {
        Some(path) => match path.value.clone().into_os_string().into_string() {
            Ok(path) => Some(Some(path)),
            Err(_) => {
                report
                    .error(
                        "invalid ca_file path. this likely a bug as it should have been \
                         caught by validation",
                    )
                    .at(path.span)
                    .emit();
                None
            }
        },
        None => Some(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AcmeServerSpec, CertStoreConfig, CertStoreSpec, PerformanceSpec, ShutdownSpec,
        TlsAutomationSpec, UpstreamSettingsSpec,
    };
    use confval::prelude::{Located, Lower};
    use std::path::PathBuf;
    use std::time::Duration;

    fn lower_server(spec: &ServerSpec) -> ServerConfig {
        let mut report = Report::new();
        let config = ServerConfig::lower(spec, &mut report);
        assert!(!report.has_errors(), "issues: {:?}", report.issues());
        config.unwrap()
    }

    #[test]
    fn wasm_defaults_when_block_omitted() {
        // Arrange
        let spec = ServerSpec::default();

        // Act
        let config = lower_server(&spec);

        // Assert
        assert_eq!(config.wasm.max_concurrent_executions, 512);
        assert_eq!(config.wasm.max_memory_bytes, 67_108_864);
    }

    #[test]
    fn server_config_from_valid_spec() {
        // Arrange
        let spec = ServerSpec::default();

        // Act
        let config = lower_server(&spec);

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
        assert!(config.upstream.connection_pool_size.is_none());
        assert!(config.performance.parallel_accepts_per_listener.is_none());
    }

    #[test]
    fn dns_refresh_interval_explicit_value() {
        // Arrange
        let spec = ServerSpec {
            dns_refresh_interval_seconds: Located::detached(60),
            ..Default::default()
        };

        // Act
        let config = lower_server(&spec);

        // Assert
        assert_eq!(config.dns_refresh_interval_seconds, 60);
    }

    #[test]
    fn shutdown_defaults_when_absent() {
        // Arrange
        let spec = ServerSpec::default();

        // Act
        let config = lower_server(&spec);

        // Assert
        assert_eq!(config.shutdown.drain_seconds, Some(10));
        assert!(config.shutdown.force_timeout_seconds.is_none());
    }

    #[test]
    fn shutdown_from_explicit_spec() {
        // Arrange
        let spec = ServerSpec {
            shutdown: Some(Located::detached(ShutdownSpec {
                drain_seconds: Some(Located::detached(30)),
                force_timeout_seconds: Some(Located::detached(60)),
            })),
            ..Default::default()
        };

        // Act
        let config = lower_server(&spec);

        // Assert
        assert_eq!(config.shutdown.drain_seconds, Some(30));
        assert_eq!(config.shutdown.force_timeout_seconds, Some(60));
    }

    #[test]
    fn performance_defaults_when_absent() {
        // Arrange
        let spec = ServerSpec::default();

        // Act
        let config = lower_server(&spec);

        // Assert
        assert!(config.performance.work_stealing);
        assert!(config.performance.parallel_accepts_per_listener.is_none());
    }

    #[test]
    fn performance_from_explicit_spec() {
        // Arrange
        let spec = ServerSpec {
            performance: Some(Located::detached(PerformanceSpec {
                work_stealing: Located::detached(false),
                parallel_accepts_per_listener: Some(Located::detached(4)),
            })),
            ..Default::default()
        };

        // Act
        let config = lower_server(&spec);

        // Assert
        assert!(!config.performance.work_stealing);
        assert_eq!(config.performance.parallel_accepts_per_listener, Some(4));
    }

    #[test]
    fn upstream_defaults_when_absent() {
        // Arrange
        let spec = ServerSpec::default();

        // Act
        let config = lower_server(&spec);

        // Assert: omitted = disabled.
        assert!(config.upstream.connection_pool_size.is_none());
        assert!(config.upstream.connection_timeout.is_none());
        assert!(config.upstream.read_timeout.is_none());
    }

    #[test]
    fn upstream_from_explicit_spec() {
        // Arrange
        let spec = ServerSpec {
            upstream: Some(Located::detached(UpstreamSettingsSpec {
                connection_pool_size: Some(Located::detached(256)),
                connection_timeout_seconds: Some(Located::detached(5)),
                read_timeout_seconds: Some(Located::detached(120)),
                source_addresses: None,
            })),
            ..Default::default()
        };

        // Act
        let config = lower_server(&spec);

        // Assert
        assert_eq!(config.upstream.connection_pool_size, Some(256));
        assert_eq!(
            config.upstream.connection_timeout,
            Some(Duration::from_secs(5))
        );
        assert_eq!(config.upstream.read_timeout, Some(Duration::from_secs(120)));
    }

    #[test]
    fn tls_automation_from_spec() {
        // Arrange
        let spec = TlsAutomationSpec {
            acme: Located::detached(AcmeServerSpec {
                directory_url: Located::detached("https://acme.example.com/dir".to_string()),
                data_dir: Located::detached(PathBuf::from("/tmp/acme")),
                contact_email: vec![Located::detached("admin@example.com".to_string())],
                ca_file: None,
            }),
            cert_store: Located::detached(CertStoreSpec::Memory),
            renew_within_days: Located::detached(30),
        };

        // Act
        let mut report = Report::new();
        let config = TlsAutomationConfig::lower(&spec, &mut report).unwrap();

        // Assert
        assert_eq!(config.renew_within_days, 30);
        assert!(matches!(config.cert_store, CertStoreConfig::Memory));
        assert_eq!(config.acme.directory_url, "https://acme.example.com/dir");
    }

    #[test]
    fn cert_store_filesystem_from_spec() {
        // Arrange
        let spec = CertStoreSpec::Filesystem {
            cert_dir: Located::detached(PathBuf::from("/etc/certs")),
        };

        // Act
        let mut report = Report::new();
        let config = CertStoreConfig::lower(&spec, &mut report).unwrap();

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
        let mut report = Report::new();
        let config = CertStoreConfig::lower(&spec, &mut report).unwrap();

        // Assert
        assert!(matches!(config, CertStoreConfig::Memory));
    }
}
