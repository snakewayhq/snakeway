use crate::types::{
    HclInt, ObservabilitySpec, PerformanceSpec, ShutdownSpec, TlsAutomationSpec, UpgradeSpec,
    UpstreamSettingsSpec, WasmSpec,
};
use confval::prelude::Located;
use serde::Serialize;
use std::path::PathBuf;

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

    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub upstream: Option<Located<UpstreamSettingsSpec>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub wasm: Option<Located<WasmSpec>>,
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
            upstream: None,
            wasm: None,
        }
    }
}

impl ServerSpec {
    /// Returns the spec with every defaultable nested block the source omitted
    /// filled in with its `Default`. `shutdown`, `upgrade`, `performance`, and
    /// `upstream` are exactly the blocks the runtime always materializes (see
    /// the `#[confval(nested, default)]` fields on `ServerConfig`), so this is
    /// the effective spec the proxy lowers from. Blocks the source wrote are
    /// left untouched.
    ///
    /// `config dump --repr=spec` serializes the spec as written, so an absent
    /// block stays absent; `--repr=populated_spec` serializes `populated()` so
    /// an operator can see the defaults the runtime will apply.
    pub fn populated(mut self) -> Self {
        self.shutdown
            .get_or_insert_with(|| Located::detached(ShutdownSpec::default()));
        self.upgrade
            .get_or_insert_with(|| Located::detached(UpgradeSpec::default()));
        self.performance
            .get_or_insert_with(|| Located::detached(PerformanceSpec::default()));
        self.upstream
            .get_or_insert_with(|| Located::detached(UpstreamSettingsSpec::default()));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CertStoreSpec, UpstreamSourceAddressesSpec};
    use confval::format::hcl::parse_hcl;
    use confval::prelude::{Report, SourceMap, Validate};
    use std::path::Path;

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
}

upstream {
  connection_pool_size = 128

  source_addresses {
    ipv4 = ["10.0.0.1"]
  }
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
        let upstream = spec.upstream.as_ref().unwrap();
        assert_eq!(
            upstream.value.connection_pool_size.as_ref().unwrap().value,
            128
        );
        let sources = upstream.value.source_addresses.as_ref().unwrap();
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
            CertStoreSpec::Filesystem { cert_dir } if cert_dir.value == Path::new("data/acme/certs")
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
        server.validate(&mut report);

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
        server.validate(&mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report.issues()[0]
                .message
                .contains("invalid config version: 2")
        );
    }

    #[test]
    fn validate_upstream_timeout_out_of_range() {
        // Arrange
        let mut report = Report::new();
        let server = ServerSpec {
            upstream: Some(Located::detached(UpstreamSettingsSpec {
                read_timeout_seconds: Some(Located::detached(99_999)),
                ..Default::default()
            })),
            ..Default::default()
        };

        // Act
        server.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("read_timeout_seconds"))
        );
    }

    #[test]
    fn validate_upstream_timeout_zero_rejected() {
        // Arrange
        let mut report = Report::new();
        let server = ServerSpec {
            upstream: Some(Located::detached(UpstreamSettingsSpec {
                connection_timeout_seconds: Some(Located::detached(0)),
                ..Default::default()
            })),
            ..Default::default()
        };

        // Act
        server.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("connection_timeout_seconds"))
        );
    }

    #[test]
    fn validate_wasm_max_concurrent_executions_out_of_range() {
        // Arrange
        let mut report = Report::new();
        let server = ServerSpec {
            wasm: Some(Located::detached(WasmSpec {
                max_concurrent_executions: Located::detached(999_999),
                ..Default::default()
            })),
            ..Default::default()
        };

        // Act
        server.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("max_concurrent_executions"))
        );
    }

    #[test]
    fn validate_wasm_max_memory_bytes_out_of_range() {
        // Arrange
        let mut report = Report::new();
        let server = ServerSpec {
            wasm: Some(Located::detached(WasmSpec {
                max_memory_bytes: Located::detached(1),
                ..Default::default()
            })),
            ..Default::default()
        };

        // Act
        server.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("max_memory_bytes"))
        );
    }

    #[test]
    fn validate_server_bad_version_suppresses_other_errors() {
        // The version gate is intentional: a bad version reports only the
        // version error and skips the v1-specific field checks, even when a
        // field is otherwise invalid (here, threads far out of range). If this
        // ever reports two errors, the early return in ServerSpec::validate was
        // removed; that is a deliberate design choice, not a bug to "fix".
        let mut report = Report::new();
        let server = ServerSpec {
            version: Located::detached(2),
            threads: Some(Located::detached(99_999)),
            ..Default::default()
        };

        server.validate(&mut report);

        assert_eq!(
            report.issues().len(),
            1,
            "expected only the version error, got: {:?}",
            report.issues()
        );
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
        server.validate(&mut report);

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
        server.validate(&mut report);

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
        server.validate(&mut report);

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
        server.validate(&mut report);

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
        server.validate(&mut report);

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
        server.validate(&mut report);

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
        server.validate(&mut report);

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
        server.validate(&mut report);

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
        server.validate(&mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn validate_upstream_source_addresses_invalid_entries() {
        // Arrange
        let mut report = Report::new();
        let server = ServerSpec {
            upstream: Some(Located::detached(UpstreamSettingsSpec {
                source_addresses: Some(Located::detached(UpstreamSourceAddressesSpec {
                    ipv4: vec![Located::detached("not an ip".to_string())],
                    ipv6: vec![Located::detached("also wrong".to_string())],
                })),
                ..Default::default()
            })),
            ..Default::default()
        };

        // Act
        server.validate(&mut report);

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
