use super::observability_spec::validate_observability;
use super::tls_automation_spec::validate_tls_automation;
use crate::types::{HclInt, ObservabilitySpec, TlsAutomationSpec};
use crate::validation::validator::validate_cert_pem;
use confval::provenance::{Located, Report};
use confval::{RangeConstraint, range_constraint};
use serde::Serialize;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;

range_constraint!(THREADS, i64, min: 1, max: 1024);
range_constraint!(DNS_REFRESH_INTERVAL_SECONDS, i64, min: 1, max: 3600, units: "seconds");
range_constraint!(SHUTDOWN_DRAIN_SECONDS, i64, min: 0, max: 300, units: "seconds");
range_constraint!(SHUTDOWN_FORCE_TIMEOUT_SECONDS, i64, min: 1, max: 300, units: "seconds");
range_constraint!(UPGRADE_MAX_RETRIES, i64, min: 1, max: 60);
range_constraint!(UPSTREAM_CONNECTION_POOL_SIZE, i64, min: 1, max: 65535);
range_constraint!(PARALLEL_ACCEPTS_PER_LISTENER, i64, min: 1, max: 64);

#[derive(Debug, Serialize, confval::Spec)]
pub struct ServerSpec {
    /// Configuration schema version
    pub version: Located<HclInt>,

    /// Optional number of worker threads - default is decided by Pingora.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads: Option<Located<HclInt>>,

    /// Optional pid file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid_file: Option<Located<PathBuf>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_file: Option<Located<PathBuf>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub tls_automation: Option<Located<TlsAutomationSpec>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub observability: Option<Located<ObservabilitySpec>>,

    #[confval(default = 30)]
    pub dns_refresh_interval_seconds: Located<HclInt>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub shutdown: Option<Located<ShutdownSpec>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub upgrade: Option<Located<UpgradeSpec>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub performance: Option<Located<PerformanceSpec>>,

    /// Local IP addresses used as the source for outbound upstream connections.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub upstream_source_addresses: Option<Located<UpstreamSourceAddressesSpec>>,
}

#[derive(Debug, Serialize, confval::Spec)]
pub struct ShutdownSpec {
    /// How long active connections are allowed to finish after a shutdown signal.
    #[confval(default = 10)]
    pub drain_seconds: Option<Located<HclInt>>,

    /// Hard ceiling on total shutdown time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_timeout_seconds: Option<Located<HclInt>>,
}

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct UpgradeSpec {
    /// Path to the Unix domain socket used for zero-drop upgrades.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sock: Option<Located<String>>,

    /// Maximum number of retries when connecting/accepting on the upgrade socket.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<Located<HclInt>>,
}

#[derive(Debug, Serialize, confval::Spec)]
pub struct PerformanceSpec {
    #[confval(default = true)]
    pub work_stealing: Located<bool>,

    /// Number of idle upstream connections kept warm per worker thread.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_connection_pool_size: Option<Located<HclInt>>,

    /// Number of parallel accept tasks per listener.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_accepts_per_listener: Option<Located<HclInt>>,
}

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct UpstreamSourceAddressesSpec {
    #[confval(default)]
    pub ipv4: Vec<Located<String>>,
    #[confval(default)]
    pub ipv6: Vec<Located<String>>,
}

impl Default for ServerSpec {
    fn default() -> Self {
        Self {
            version: Located::detached(1),
            threads: None,
            pid_file: None,
            ca_file: None,
            tls_automation: None,
            observability: None,
            dns_refresh_interval_seconds: Located::detached(30),
            shutdown: None,
            upgrade: None,
            performance: None,
            upstream_source_addresses: None,
        }
    }
}

impl Default for ShutdownSpec {
    fn default() -> Self {
        Self {
            drain_seconds: Some(Located::detached(10)),
            force_timeout_seconds: None,
        }
    }
}

impl Default for PerformanceSpec {
    fn default() -> Self {
        Self {
            work_stealing: Located::detached(true),
            upstream_connection_pool_size: None,
            parallel_accepts_per_listener: None,
        }
    }
}

/// Entity-level validation for the server section. Runs after parsing (or
/// after programmatic construction), so it must not assume a source file
/// exists; spans come from the `Located` values themselves.
pub fn validate_server(spec: &ServerSpec, report: &mut Report) {
    if spec.version.value != 1 {
        report
            .error(format!("invalid config version: {}", spec.version.value))
            .at(spec.version.span)
            .help(
                "This version of Snakeway is not compatible with this config file. \
                 Please upgrade Snakeway.",
            )
            .emit();
        return;
    }

    if let Some(pid_file) = &spec.pid_file
        && let Some(parent) = pid_file.value.parent()
    {
        if !parent.exists() {
            report
                .error(format!(
                    "pid file parent directory does not exist: {}",
                    pid_file.value.display()
                ))
                .at(pid_file.span)
                .emit();
        } else if !parent.is_dir() {
            report
                .error(format!(
                    "pid file parent is not a directory: {}",
                    pid_file.value.display()
                ))
                .at(pid_file.span)
                .emit();
        }
    }

    if let Some(ca_file) = &spec.ca_file
        && let Err(e) = validate_cert_pem(&ca_file.value)
    {
        report
            .error(format!("server CA file is invalid: {}", e))
            .at(ca_file.span)
            .emit();
    }

    if let Some(threads) = &spec.threads {
        THREADS.check_located(threads, "threads", report);
    }

    DNS_REFRESH_INTERVAL_SECONDS.check_located(
        &spec.dns_refresh_interval_seconds,
        "dns_refresh_interval_seconds",
        report,
    );

    if let Some(tls_automation) = &spec.tls_automation {
        validate_tls_automation(&tls_automation.value, report);
    }

    if let Some(observability) = &spec.observability {
        validate_observability(&observability.value, report);
    }

    if let Some(shutdown) = &spec.shutdown {
        if let Some(drain) = &shutdown.value.drain_seconds {
            SHUTDOWN_DRAIN_SECONDS.check_located(drain, "drain_seconds", report);
        }
        if let Some(timeout) = &shutdown.value.force_timeout_seconds {
            SHUTDOWN_FORCE_TIMEOUT_SECONDS.check_located(timeout, "force_timeout_seconds", report);
        }
    }

    if let Some(upgrade) = &spec.upgrade
        && let Some(retries) = &upgrade.value.max_retries
    {
        UPGRADE_MAX_RETRIES.check_located(retries, "max_retries", report);
    }

    if let Some(performance) = &spec.performance {
        if let Some(pool_size) = &performance.value.upstream_connection_pool_size {
            UPSTREAM_CONNECTION_POOL_SIZE.check_located(
                pool_size,
                "upstream_connection_pool_size",
                report,
            );
        }
        if let Some(accepts) = &performance.value.parallel_accepts_per_listener {
            PARALLEL_ACCEPTS_PER_LISTENER.check_located(
                accepts,
                "parallel_accepts_per_listener",
                report,
            );
        }
    }

    if let Some(source_addrs) = &spec.upstream_source_addresses {
        for addr in &source_addrs.value.ipv4 {
            if addr.value.parse::<Ipv4Addr>().is_err() {
                report
                    .error(format!(
                        "invalid upstream_source_addresses.ipv4 entry: \"{}\" is not a valid IPv4 address",
                        addr.value
                    ))
                    .at(addr.span)
                    .emit();
            }
        }
        for addr in &source_addrs.value.ipv6 {
            if addr.value.parse::<Ipv6Addr>().is_err() {
                report
                    .error(format!(
                        "invalid upstream_source_addresses.ipv6 entry: \"{}\" is not a valid IPv6 address",
                        addr.value
                    ))
                    .at(addr.span)
                    .emit();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CertStoreSpec;
    use confval::hcl::parse_hcl;
    use confval::provenance::SourceMap;

    fn parse_server(input: &str) -> (Report, Option<ServerSpec>) {
        let mut sources = SourceMap::new();
        let mut report = Report::new();
        let id = sources.add("snakeway.hcl", input);
        let spec = parse_hcl::<ServerSpec>(&sources, id, &mut report);
        (report, spec)
    }

    #[test]
    fn parse_minimal_server() {
        // Arrange
        let input = "version = 1\n";

        // Act
        let (report, spec) = parse_server(input);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let spec = spec.unwrap();
        assert_eq!(spec.version.value, 1);
        assert_eq!(spec.dns_refresh_interval_seconds.value, 30);
        assert!(spec.dns_refresh_interval_seconds.span.is_detached());
    }

    #[test]
    fn parse_full_server_with_blocks() {
        // Arrange
        let input = r#"version = 1
threads = 4
pid_file = "/tmp/snakeway.pid"
dns_refresh_interval_seconds = 60

shutdown {
  drain_seconds = 20
}

upgrade {
  sock = "/tmp/upgrade.sock"
  max_retries = 5
}

performance {
  work_stealing = false
  upstream_connection_pool_size = 128
}

upstream_source_addresses {
  ipv4 = ["10.0.0.1"]
}
"#;

        // Act
        let (report, spec) = parse_server(input);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let spec = spec.unwrap();
        assert_eq!(spec.threads.as_ref().unwrap().value, 4);
        assert_eq!(
            spec.pid_file.as_ref().unwrap().value,
            PathBuf::from("/tmp/snakeway.pid")
        );
        assert_eq!(spec.dns_refresh_interval_seconds.value, 60);
        let shutdown = spec.shutdown.as_ref().unwrap();
        assert_eq!(shutdown.value.drain_seconds.as_ref().unwrap().value, 20);
        let upgrade = spec.upgrade.as_ref().unwrap();
        assert_eq!(
            upgrade.value.sock.as_ref().unwrap().value,
            "/tmp/upgrade.sock"
        );
        let performance = spec.performance.as_ref().unwrap();
        assert!(!performance.value.work_stealing.value);
        let sources = spec.upstream_source_addresses.as_ref().unwrap();
        assert_eq!(sources.value.ipv4[0].value, "10.0.0.1");
    }

    /// Nested structures are accepted in both HCL spellings: blocks and
    /// object-valued attributes. Real configs mix them freely.
    #[test]
    fn parse_accepts_object_syntax_for_nested_structures() {
        // Arrange
        let input = r#"version = 1
threads = 1

performance {
  work_stealing = true
}

tls_automation = {
  acme = {
    directory_url = "https://localhost:14000/dir"
    contact_email = ["admin@example.com"]
    ca_file       = "./pebble-ca.pem"
    data_dir      = "data/acme/orders"
  }

  renew_within_days = 30

  cert_store = {
    type     = "filesystem"
    cert_dir = "data/acme/certs"
  }
}

observability {
  otel {
    enable         = false
    endpoint       = "http://localhost:4317"
    service_name   = "snakeway"
    sampling_ratio = 0.01
  }
}
"#;

        // Act
        let (report, spec) = parse_server(input);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let spec = spec.unwrap();
        let tls = spec.tls_automation.as_ref().unwrap();
        assert_eq!(
            tls.value.acme.value.directory_url.value,
            "https://localhost:14000/dir"
        );
        assert!(matches!(
            &tls.value.cert_store.value,
            CertStoreSpec::Filesystem { cert_dir } if cert_dir.value == PathBuf::from("data/acme/certs")
        ));
        let span = tls.value.acme.value.directory_url.span;
        assert_eq!(
            &input[span.start as usize..span.end as usize],
            "\"https://localhost:14000/dir\""
        );
        let otel = spec
            .observability
            .as_ref()
            .unwrap()
            .value
            .otel
            .as_ref()
            .unwrap();
        assert_eq!(otel.value.sampling_ratio.value, 0.01);
    }

    #[test]
    fn parse_shutdown_drain_defaults_to_ten_when_block_present() {
        // Arrange
        let input = "version = 1\n\nshutdown {\n}\n";

        // Act
        let (report, spec) = parse_server(input);

        // Assert
        assert!(!report.has_issues());
        let shutdown = spec.unwrap().shutdown.unwrap();
        assert_eq!(shutdown.value.drain_seconds.unwrap().value, 10);
    }

    #[test]
    fn parse_missing_version_is_reported() {
        // Arrange
        let input = "threads = 4\n";

        // Act
        let (report, spec) = parse_server(input);

        // Assert
        assert!(spec.is_none());
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "missing required field: version")
        );
    }

    #[test]
    fn parse_unknown_field_is_reported() {
        // Arrange
        let input = "version = 1\nthreds = 4\n";

        // Act
        let (report, _) = parse_server(input);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "unknown field: threds")
        );
    }

    #[test]
    fn validate_server_version_valid() {
        // Arrange
        let mut report = Report::new();
        let server = ServerSpec::default();

        // Act
        validate_server(&server, &mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn validate_server_version_invalid() {
        // Arrange
        let mut report = Report::new();
        let server = ServerSpec {
            version: Located::detached(2),
            ..Default::default()
        };

        // Act
        validate_server(&server, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report.issues()[0]
                .message
                .contains("invalid config version: 2")
        );
    }

    #[test]
    fn validate_server_valid_config() {
        // Arrange
        let mut report = Report::new();
        let server = ServerSpec {
            threads: Some(Located::detached(4)),
            ..Default::default()
        };

        // Act
        validate_server(&server, &mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn validate_server_pid_file_parent_dir_does_not_exist() {
        // Arrange
        let mut report = Report::new();
        let server = ServerSpec {
            pid_file: Some(Located::detached(PathBuf::from(
                "/non/existent/path/snakeway.pid",
            ))),
            ..Default::default()
        };

        // Act
        validate_server(&server, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report.issues()[0]
                .message
                .contains("pid file parent directory does not exist")
        );
    }

    #[test]
    fn validate_server_pid_file_parent_is_not_a_dir() {
        // Arrange
        let mut report = Report::new();
        let dir = tempfile::tempdir().unwrap();
        let fake_parent = dir.path().join("not_a_dir");
        std::fs::write(&fake_parent, "hello").unwrap();
        let server = ServerSpec {
            pid_file: Some(Located::detached(fake_parent.join("snakeway.pid"))),
            ..Default::default()
        };

        // Act
        validate_server(&server, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message.contains("pid file parent is not a directory"))
        );
    }

    #[test]
    fn validate_server_ca_file_does_not_exist() {
        // Arrange
        let ca_file = PathBuf::from("/non/existent/ca.pem");
        let mut report = Report::new();
        let server = ServerSpec {
            ca_file: Some(Located::detached(ca_file.clone())),
            ..Default::default()
        };
        let expected = format!(
            "server CA file is invalid: file does not exist: {}",
            ca_file.to_string_lossy()
        );

        // Act
        validate_server(&server, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(report.issues()[0].message.contains(&expected));
    }

    #[test]
    fn validate_server_threads_too_low() {
        // Arrange
        let mut report = Report::new();
        let server = ServerSpec {
            threads: Some(Located::detached(0)),
            ..Default::default()
        };

        // Act
        validate_server(&server, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report.issues()[0]
                .message
                .contains("threads must be at least 1")
        );
    }

    #[test]
    fn validate_server_threads_too_high() {
        // Arrange
        let mut report = Report::new();
        let server = ServerSpec {
            threads: Some(Located::detached(1025)),
            ..Default::default()
        };

        // Act
        validate_server(&server, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report.issues()[0]
                .message
                .contains("threads must be at most 1024")
        );
    }

    #[test]
    fn validate_dns_refresh_interval_valid() {
        // Arrange
        let mut report = Report::new();
        let server = ServerSpec {
            dns_refresh_interval_seconds: Located::detached(60),
            ..Default::default()
        };

        // Act
        validate_server(&server, &mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn validate_dns_refresh_interval_too_high() {
        // Arrange
        let mut report = Report::new();
        let server = ServerSpec {
            dns_refresh_interval_seconds: Located::detached(3601),
            ..Default::default()
        };

        // Act
        validate_server(&server, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report.issues()[0]
                .message
                .contains("dns_refresh_interval_seconds must be at most 3600")
        );
    }

    #[test]
    fn validate_server_valid_pid_and_ca_files() {
        // Arrange
        let mut report = Report::new();
        let dir = tempfile::tempdir().unwrap();
        let pid_dir = dir.path().join("pid");
        std::fs::create_dir(&pid_dir).unwrap();
        let ca_file = dir.path().join("ca.pem");
        std::fs::write(&ca_file, "dummy").unwrap();
        let server = ServerSpec {
            pid_file: Some(Located::detached(pid_dir.join("snakeway.pid"))),
            ca_file: Some(Located::detached(ca_file)),
            ..Default::default()
        };

        // Act
        validate_server(&server, &mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn validate_upstream_source_addresses_invalid_entries() {
        // Arrange
        let mut report = Report::new();
        let server = ServerSpec {
            upstream_source_addresses: Some(Located::detached(UpstreamSourceAddressesSpec {
                ipv4: vec![Located::detached("not an ip".to_string())],
                ipv6: vec![Located::detached("also wrong".to_string())],
            })),
            ..Default::default()
        };

        // Act
        validate_server(&server, &mut report);

        // Assert
        assert_eq!(report.issues().len(), 2);
        assert!(
            report.issues()[0]
                .message
                .contains("is not a valid IPv4 address")
        );
        assert!(
            report.issues()[1]
                .message
                .contains("is not a valid IPv6 address")
        );
    }
}
