use super::admin_auth_spec::{AdminAuthSpec, report_admin_auth_missing, validate_admin_auth};
use crate::resolution::ResolveError;
use crate::types::specification::ingress::bind::report_invalid_port;
use crate::types::specification::ingress::bind::validate_tls_termination;
use crate::types::{BindInterfaceSpec, HclInt, TlsTerminationSpec};
use crate::validation::ConfigError;
use crate::validation::validator::is_valid_port;
use confval::provenance::{Located, Report, Span};
use serde::Serialize;
use std::net::SocketAddr;

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct BindAdminSpec {
    pub interface: Located<String>,
    pub port: Located<HclInt>,
    #[confval(nested)]
    pub tls: Located<TlsTerminationSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub auth: Option<Located<AdminAuthSpec>>,
}

impl BindAdminSpec {
    pub fn resolve(&self) -> Result<SocketAddr, ResolveError> {
        let interface: BindInterfaceSpec = self
            .interface
            .value
            .as_str()
            .try_into()
            .map_err(|e: ConfigError| ResolveError::InvalidInterface(e.to_string()))?;

        Ok(SocketAddr::new(interface.as_ip(), self.port.value as u16))
    }
}

pub fn validate_bind_admin(spec: &BindAdminSpec, span: Span, report: &mut Report) {
    if !is_valid_port(spec.port.value) {
        report_invalid_port(&spec.port, report);
    }

    match &spec.tls.value {
        TlsTerminationSpec::Manual { .. } => {
            validate_tls_termination(&spec.tls.value, report);
        }
        TlsTerminationSpec::Acme { .. } => {
            report
                .error("admin bind does not support ACME TLS")
                .at(spec.tls.span)
                .emit();
        }
    }

    match &spec.auth {
        Some(auth) => validate_admin_auth(&auth.value, auth.span, report),
        None => report_admin_auth_missing(span, report),
    }

    let maybe_interface: Result<BindInterfaceSpec, _> = spec.interface.value.as_str().try_into();

    match maybe_interface {
        Ok(BindInterfaceSpec::Ip(ip)) if ip.is_unspecified() => {
            report
                .error("invalid bind address: 0.0.0.0 or ::")
                .at(spec.interface.span)
                .emit();
        }
        Ok(_) => {
            // All good.
        }
        Err(_) => {
            report
                .error(format!("invalid bind address: {}", spec.interface.value))
                .at(spec.interface.span)
                .emit();
            return;
        }
    }

    if let Ok(interface) = maybe_interface
        && matches!(interface, BindInterfaceSpec::All)
    {
        report
            .error("admin API cannot bind to all interfaces")
            .at(spec.interface.span)
            .help("Use loopback or a specific IP address.")
            .emit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ACME_CHALLENGE_HTTP01, BearerAuthSpec};
    use rcgen::generate_simple_self_signed;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::{NamedTempFile, tempdir};

    const TEST_TOKEN: &str = "a9f1c38de4b67029c5d1e97f4a0ebac12d3b8ffc84e1d27a05f6cb9e83d21a04";

    fn write_token_file(contents: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("tempfile");
        f.write_all(contents.as_bytes()).expect("write");
        f
    }

    fn valid_bearer_auth(token_file: PathBuf) -> Option<Located<AdminAuthSpec>> {
        Some(Located::detached(AdminAuthSpec {
            bearer: Some(Located::detached(BearerAuthSpec {
                token_file: Located::detached(token_file),
            })),
        }))
    }

    fn minimal_bind_admin() -> BindAdminSpec {
        BindAdminSpec {
            interface: Located::detached("loopback".to_string()),
            port: Located::detached(9000),
            ..Default::default()
        }
    }

    fn validate(spec: &BindAdminSpec) -> Report {
        let mut report = Report::new();
        validate_bind_admin(spec, Span::detached(), &mut report);
        report
    }

    #[test]
    fn acme_tls_not_supported() {
        // Arrange
        let bind_admin = BindAdminSpec {
            tls: Located::detached(TlsTerminationSpec::Acme {
                domains: vec![Located::detached("example.com".to_string())],
                challenge: Located::detached(ACME_CHALLENGE_HTTP01.to_string()),
            }),
            ..minimal_bind_admin()
        };

        // Act
        let report = validate(&bind_admin);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.to_lowercase().contains("acme"))
        );
    }

    #[test]
    fn unspecified_ip_rejected() {
        // Arrange
        let bind_admin = BindAdminSpec {
            interface: Located::detached("0.0.0.0".to_string()),
            ..minimal_bind_admin()
        };

        // Act
        let report = validate(&bind_admin);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("0.0.0.0"))
        );
    }

    #[test]
    fn invalid_bind_addr() {
        // Arrange
        let bind_admin = BindAdminSpec {
            interface: Located::detached("bad-addr".to_string()),
            ..minimal_bind_admin()
        };

        // Act
        let report = validate(&bind_admin);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "invalid bind address: bad-addr")
        );
    }

    #[test]
    fn cannot_bind_to_all_interfaces() {
        // Arrange
        let dir = tempdir().expect("failed to create temp dir");

        let cert = generate_simple_self_signed(vec!["localhost".into()])
            .expect("failed to generate self-signed cert");
        let cert_pem = cert.cert.pem();
        let key_pem = cert.signing_key.serialize_pem();

        let cert_path = dir.path().join("tmp-cert.pem");
        let mut cert_file = File::create(&cert_path).expect("failed to create cert file");
        cert_file
            .write_all(cert_pem.as_bytes())
            .expect("failed to write cert");

        let key_path = dir.path().join("tmp-key.pem");
        let mut key_file = File::create(&key_path).expect("failed to create key file");
        key_file
            .write_all(key_pem.as_bytes())
            .expect("failed to write key");

        let token_file = write_token_file(&format!("{}\n", TEST_TOKEN));

        let bind_admin = BindAdminSpec {
            interface: Located::detached("all".to_string()),
            port: Located::detached(9000),
            tls: Located::detached(TlsTerminationSpec::Manual {
                cert: Located::detached(cert_path),
                key: Located::detached(key_path),
            }),
            auth: valid_bearer_auth(token_file.path().to_path_buf()),
        };

        // Act
        let report = validate(&bind_admin);

        // Assert
        assert_eq!(report.issues().len(), 1);
        assert_eq!(
            report.issues()[0].message,
            "admin API cannot bind to all interfaces"
        );
        assert_eq!(
            report.issues()[0].help.as_deref(),
            Some("Use loopback or a specific IP address.")
        );
    }

    #[test]
    fn missing_auth_block_produces_error() {
        // Arrange
        let bind_admin = minimal_bind_admin();

        // Act
        let report = validate(&bind_admin);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "bind_admin.auth is required")
        );
    }

    #[test]
    fn explicit_empty_auth_block_produces_error() {
        // Arrange
        let bind_admin = BindAdminSpec {
            auth: Some(Located::detached(AdminAuthSpec { bearer: None })),
            ..minimal_bind_admin()
        };

        // Act
        let report = validate(&bind_admin);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "bind_admin.auth is required")
        );
    }

    #[test]
    fn bearer_token_file_missing_produces_error() {
        // Arrange
        let bind_admin = BindAdminSpec {
            auth: valid_bearer_auth(PathBuf::from("/non/existent/tokens")),
            ..minimal_bind_admin()
        };

        // Act
        let report = validate(&bind_admin);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("bearer token_file could not be read"))
        );
    }

    #[test]
    fn bearer_token_file_empty_produces_error() {
        // Arrange
        let token_file = write_token_file("");
        let bind_admin = BindAdminSpec {
            auth: valid_bearer_auth(token_file.path().to_path_buf()),
            ..minimal_bind_admin()
        };

        // Act
        let report = validate(&bind_admin);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("bearer token_file is empty"))
        );
    }

    #[test]
    fn bearer_token_too_short_produces_error() {
        // Arrange
        let token_file = write_token_file("short\n");
        let bind_admin = BindAdminSpec {
            auth: valid_bearer_auth(token_file.path().to_path_buf()),
            ..minimal_bind_admin()
        };

        // Act
        let report = validate(&bind_admin);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("minimum is"))
        );
    }

    #[test]
    fn bearer_comment_line_produces_error() {
        // Arrange
        let token_file = write_token_file(&format!("# comment\n{}\n", TEST_TOKEN));
        let bind_admin = BindAdminSpec {
            auth: valid_bearer_auth(token_file.path().to_path_buf()),
            ..minimal_bind_admin()
        };

        // Act
        let report = validate(&bind_admin);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("comments are not permitted"))
        );
    }

    #[test]
    fn bearer_duplicate_tokens_emit_warning_not_error() {
        // Arrange
        let token_file = write_token_file(&format!("{}\n{}\n", TEST_TOKEN, TEST_TOKEN));
        let bind_admin = BindAdminSpec {
            auth: valid_bearer_auth(token_file.path().to_path_buf()),
            ..minimal_bind_admin()
        };

        // Act
        let report = validate(&bind_admin);

        // Assert
        assert!(
            !report
                .issues()
                .iter()
                .any(|i| i.severity == confval::Severity::Error
                    && i.message.contains("token_file")),
            "issues: {:?}",
            report.issues()
        );
        assert!(report.has_warnings());
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("duplicate token"))
        );
    }

    #[test]
    fn valid_auth_block_produces_no_errors() {
        // Arrange
        let token_file = write_token_file(&format!("{}\n", TEST_TOKEN));
        let bind_admin = BindAdminSpec {
            auth: valid_bearer_auth(token_file.path().to_path_buf()),
            ..minimal_bind_admin()
        };

        // Act
        let report = validate(&bind_admin);

        // Assert
        assert!(
            !report
                .issues()
                .iter()
                .any(|i| i.message.contains("auth") || i.message.contains("token_file")),
            "issues: {:?}",
            report.issues()
        );
    }
}
