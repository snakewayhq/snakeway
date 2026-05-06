use crate::types::{
    AcmeServerSpec, CertStoreSpec, ObservabilitySpec, OtelSpec, ServerSpec, TlsAutomationSpec,
};

use super::{
    AcmeServerConfig, CertStoreConfig, ObservabilityConfig, OtelConfig, ServerConfig,
    TlsAutomationConfig,
};

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
            dns_refresh_interval_seconds: spec.dns_refresh_interval_seconds,
            upgrade_sock: spec.upgrade_sock,
            upgrade_max_retries: spec.upgrade_max_retries,
            grace_period_seconds: spec.grace_period_seconds,
            graceful_shutdown_timeout_seconds: spec.graceful_shutdown_timeout_seconds,
        })
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
        assert!(config.work_stealing);
        assert!(config.ca_file.is_none());
        assert!(config.tls_automation.is_none());
        assert!(config.observability.is_none());
        assert_eq!(config.dns_refresh_interval_seconds, 30);
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
