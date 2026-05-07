use crate::types::{
    AcmeServerSpec, CertStoreSpec, ObservabilitySpec, OtelSpec, PerformanceSpec, ServerSpec,
    ShutdownSpec, TlsAutomationSpec, UpgradeSpec, UpstreamSourceAddressesSpec,
};

use super::{
    AcmeServerConfig, CertStoreConfig, ObservabilityConfig, OtelConfig, PerformanceConfig,
    ServerConfig, ShutdownConfig, TlsAutomationConfig, UpgradeConfig,
    UpstreamSourceAddressesConfig,
};

impl TryFrom<ServerSpec> for ServerConfig {
    type Error = String;
    fn try_from(spec: ServerSpec) -> Result<Self, Self::Error> {
        Ok(Self {
            version: spec.version,
            threads: spec.threads,
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
            dns_refresh_interval_seconds: spec.dns_refresh_interval_seconds,
            shutdown: spec.shutdown.into(),
            upgrade: spec.upgrade.into(),
            performance: spec.performance.into(),
            upstream_source_addresses: spec.upstream_source_addresses.map(Into::into),
        })
    }
}

impl From<Option<ShutdownSpec>> for ShutdownConfig {
    fn from(spec: Option<ShutdownSpec>) -> Self {
        match spec {
            Some(s) => Self {
                drain_seconds: s.drain_seconds,
                force_timeout_seconds: s.force_timeout_seconds,
            },
            None => Self {
                drain_seconds: Some(10),
                force_timeout_seconds: None,
            },
        }
    }
}

impl From<Option<UpgradeSpec>> for UpgradeConfig {
    fn from(spec: Option<UpgradeSpec>) -> Self {
        match spec {
            Some(s) => Self {
                sock: s.sock,
                max_retries: s.max_retries,
            },
            None => Self {
                sock: None,
                max_retries: None,
            },
        }
    }
}

impl From<Option<PerformanceSpec>> for PerformanceConfig {
    fn from(spec: Option<PerformanceSpec>) -> Self {
        match spec {
            Some(s) => Self {
                work_stealing: s.work_stealing,
                upstream_connection_pool_size: s.upstream_connection_pool_size,
                parallel_accepts_per_listener: s.parallel_accepts_per_listener,
            },
            None => Self {
                work_stealing: true,
                upstream_connection_pool_size: None,
                parallel_accepts_per_listener: None,
            },
        }
    }
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

impl From<CertStoreSpec> for CertStoreConfig {
    fn from(spec: CertStoreSpec) -> Self {
        match spec {
            CertStoreSpec::Filesystem { cert_dir } => Self::Filesystem { cert_dir },
            CertStoreSpec::Memory => Self::Memory,
        }
    }
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
            sampling_ratio: spec.sampling_ratio,
        }
    }
}

impl From<UpstreamSourceAddressesSpec> for UpstreamSourceAddressesConfig {
    fn from(spec: UpstreamSourceAddressesSpec) -> Self {
        Self {
            ipv4: spec.ipv4,
            ipv6: spec.ipv6,
        }
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
