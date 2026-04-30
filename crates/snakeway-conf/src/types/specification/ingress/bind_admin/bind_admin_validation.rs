use crate::types::{
    AdminAuthSpec, BearerAuthSpec, BindAdminSpec, BindInterfaceSpec, Origin, TlsTerminationSpec,
};
use crate::validation::validator::{
    MIN_TOKEN_LENGTH, TokenFileIssue, is_valid_port, parse_token_file,
};
use crate::validation::{ValidateSpec, ValidationReport};

impl ValidateSpec for BindAdminSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        // Port validation.
        if !is_valid_port(self.port) {
            report.invalid_port(self.port, origin);
        }

        // TLS cert/key/acme validation.
        match &self.tls {
            TlsTerminationSpec::Manual { .. } => {
                self.tls.validate(origin, report);
            }
            TlsTerminationSpec::Acme { .. } => {
                report.admin_bind_does_not_support_acme(origin);
            }
        }

        // Auth validation.
        validate_admin_auth(&self.auth, origin, report);

        let maybe_interface: Result<BindInterfaceSpec, _> = self.interface.clone().try_into();

        match maybe_interface {
            Ok(BindInterfaceSpec::Ip(ip)) if ip.is_unspecified() => {
                // Note: an unspecified IP address is "0.0.0.0" or "::"
                // which resolves to all interfaces.
                // Binding to all interfaces exposes the admin API to the network,
                // which is not allowed.
                report.invalid_bind_addr("0.0.0.0 or ::", origin);
            }
            Ok(_) => {
                // All good.
            }
            Err(_) => {
                report.invalid_bind_addr(&self.interface.to_string(), origin);
                return;
            }
        }

        if let Ok(interface) = maybe_interface
            && matches!(interface, BindInterfaceSpec::All)
        {
            // This check might look redundant, but it's not.
            // There are two ways to bind to all interfaces:
            // 1. Use "all" enum option.
            // 2. Use a specific IP address.
            report.error(
                "admin API cannot bind to all interfaces".to_string(),
                origin,
                Some("Use loopback or a specific IP address.".to_string()),
            );
        }
    }
}

fn validate_admin_auth(auth: &AdminAuthSpec, origin: &Origin, report: &mut ValidationReport) {
    let Some(bearer) = &auth.bearer else {
        if auth_is_empty(auth) {
            report.admin_auth_missing(origin);
        } else {
            report.admin_auth_bearer_missing(origin);
        }
        return;
    };

    validate_bearer_auth(bearer, origin, report);
}

fn auth_is_empty(auth: &AdminAuthSpec) -> bool {
    auth.bearer.is_none()
}

fn validate_bearer_auth(bearer: &BearerAuthSpec, origin: &Origin, report: &mut ValidationReport) {
    let path = bearer.token_file.as_path();

    // token_file must be set (reject the default empty path directly).
    if path.as_os_str().is_empty() {
        report.admin_auth_bearer_token_file_io_error(path, "token_file path is empty", origin);
        return;
    }

    // Parse the file and surface every issue.
    let outcome = parse_token_file(path);

    for err in &outcome.errors {
        match err {
            TokenFileIssue::FileIoError(msg) => {
                report.admin_auth_bearer_token_file_io_error(path, msg, origin);
            }
            TokenFileIssue::EmptyFile => {
                report.admin_auth_bearer_token_file_empty(path, origin);
            }
            TokenFileIssue::EmptyLine(line) => {
                report.admin_auth_bearer_empty_line(path, *line, origin);
            }
            TokenFileIssue::CommentNotAllowed(line) => {
                report.admin_auth_bearer_comment_line(path, *line, origin);
            }
            TokenFileIssue::TokenTooShort { line, len } => {
                report.admin_auth_bearer_token_too_short(
                    path,
                    *line,
                    *len,
                    MIN_TOKEN_LENGTH,
                    origin,
                );
            }
            TokenFileIssue::DuplicateToken { .. } => {
                // Duplicates are warnings and are enumerated below.
            }
        }
    }

    for warn in &outcome.warnings {
        if let TokenFileIssue::DuplicateToken {
            line,
            first_seen_line,
        } = warn
        {
            report.admin_auth_bearer_duplicate_token(path, *line, *first_seen_line, origin);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{
        AdminAuthSpec, BearerAuthSpec, BindAdminSpec, BindInterfaceInput, Origin,
        TlsTerminationSpec,
    };
    use crate::validation::{ValidateSpec, ValidationReport};
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

    fn valid_bearer_auth(token_file: PathBuf) -> AdminAuthSpec {
        AdminAuthSpec {
            bearer: Some(BearerAuthSpec {
                token_file,
                origin: Default::default(),
            }),
            origin: Default::default(),
        }
    }

    #[test]
    fn acme_tls_not_supported() {
        // Arrange
        let mut report = ValidationReport::default();
        let bind_admin = BindAdminSpec {
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 9000,
            tls: TlsTerminationSpec::Acme {
                domains: vec!["example.com".to_string()],
                challenge: Default::default(),
            },
            ..Default::default()
        };
        let origin = Origin::test("bind_admin");

        // Act
        bind_admin.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.to_lowercase().contains("acme"))
        );
    }

    #[test]
    fn unspecified_ip_rejected() {
        // Arrange
        let mut report = ValidationReport::default();
        let bind_admin = BindAdminSpec {
            interface: BindInterfaceInput::Keyword("0.0.0.0".to_string()),
            port: 9000,
            ..Default::default()
        };
        let origin = Origin::test("bind_admin");

        // Act
        bind_admin.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(report.errors.iter().any(|e| e.message.contains("0.0.0.0")));
    }

    #[test]
    fn invalid_bind_addr() {
        // Arrange
        let mut report = ValidationReport::default();
        let bind_admin = BindAdminSpec {
            interface: BindInterfaceInput::Keyword("bad-addr".to_string()),
            port: 9000,
            ..Default::default()
        };
        let origin = Origin::test("bind_admin");

        // Act
        bind_admin.validate(&origin, &mut report);

        // Assert
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message == "invalid bind address: bad-addr")
        );
    }

    #[test]
    fn cannot_bind_to_all_interfaces() {
        // Arrange
        let mut report = ValidationReport::default();
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
            interface: BindInterfaceInput::Keyword("all".to_string()),
            port: 9000,
            tls: TlsTerminationSpec::Manual {
                cert: cert_path,
                key: key_path,
            },
            auth: valid_bearer_auth(token_file.path().to_path_buf()),
            ..Default::default()
        };
        let origin = Origin::test("bind_admin");

        // Act
        bind_admin.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors.len(), 1);
        assert_eq!(
            report.errors[0].message,
            "admin API cannot bind to all interfaces"
        );
        assert_eq!(
            report.errors[0].help.as_deref(),
            Some("Use loopback or a specific IP address.")
        );
    }

    #[test]
    fn missing_auth_block_produces_error() {
        // Arrange
        let mut report = ValidationReport::default();
        let bind_admin = BindAdminSpec {
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 9000,
            ..Default::default()
        };
        let origin = Origin::test("bind_admin");

        // Act
        bind_admin.validate(&origin, &mut report);

        // Assert
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message == "bind_admin.auth is required"),
            "expected admin_auth_missing error; got: {:?}",
            report.errors
        );
    }

    #[test]
    fn missing_bearer_scheme_produces_error() {
        // Arrange
        let mut report = ValidationReport::default();
        let bind_admin = BindAdminSpec {
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 9000,
            auth: AdminAuthSpec::default(),
            ..Default::default()
        };
        let origin = Origin::test("bind_admin");

        // Act
        bind_admin.validate(&origin, &mut report);

        // Assert: default auth is bearer=None, which triggers admin_auth_missing
        // (empty block), not bearer_missing.
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message == "bind_admin.auth is required")
        );
    }

    #[test]
    fn bearer_token_file_missing_produces_error() {
        // Arrange
        let mut report = ValidationReport::default();
        let bind_admin = BindAdminSpec {
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 9000,
            auth: valid_bearer_auth(PathBuf::from("/nonexistent/path/to/tokens.txt")),
            ..Default::default()
        };
        let origin = Origin::test("bind_admin");

        // Act
        bind_admin.validate(&origin, &mut report);

        // Assert
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("token_file could not be read")),
            "expected io error; got: {:?}",
            report.errors
        );
    }

    #[test]
    fn bearer_token_file_empty_produces_error() {
        // Arrange
        let mut report = ValidationReport::default();
        let token_file = write_token_file("");
        let bind_admin = BindAdminSpec {
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 9000,
            auth: valid_bearer_auth(token_file.path().to_path_buf()),
            ..Default::default()
        };
        let origin = Origin::test("bind_admin");

        // Act
        bind_admin.validate(&origin, &mut report);

        // Assert
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("token_file is empty"))
        );
    }

    #[test]
    fn bearer_token_too_short_produces_error() {
        // Arrange
        let mut report = ValidationReport::default();
        let token_file = write_token_file("password123\n");
        let bind_admin = BindAdminSpec {
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 9000,
            auth: valid_bearer_auth(token_file.path().to_path_buf()),
            ..Default::default()
        };
        let origin = Origin::test("bind_admin");

        // Act
        bind_admin.validate(&origin, &mut report);

        // Assert
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("minimum is 32"))
        );
    }

    #[test]
    fn bearer_comment_line_produces_error() {
        // Arrange
        let mut report = ValidationReport::default();
        let token_file = write_token_file(&format!("# comment\n{}\n", TEST_TOKEN));
        let bind_admin = BindAdminSpec {
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 9000,
            auth: valid_bearer_auth(token_file.path().to_path_buf()),
            ..Default::default()
        };
        let origin = Origin::test("bind_admin");

        // Act
        bind_admin.validate(&origin, &mut report);

        // Assert
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("comments are not permitted"))
        );
    }

    #[test]
    fn bearer_duplicate_tokens_emit_warning_not_error() {
        // Arrange
        let mut report = ValidationReport::default();
        let token_file = write_token_file(&format!("{t}\n{t}\n", t = TEST_TOKEN));
        let bind_admin = BindAdminSpec {
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 9000,
            auth: valid_bearer_auth(token_file.path().to_path_buf()),
            ..Default::default()
        };
        let origin = Origin::test("bind_admin");

        // Act
        bind_admin.validate(&origin, &mut report);

        // Assert
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.message.contains("duplicate token")),
            "expected duplicate warning; got warnings: {:?}, errors: {:?}",
            report.warnings,
            report.errors
        );
    }

    #[test]
    fn valid_auth_block_produces_no_errors() {
        // Arrange
        let mut report = ValidationReport::default();
        let dir = tempdir().expect("failed to create temp dir");
        let cert = generate_simple_self_signed(vec!["localhost".into()])
            .expect("failed to generate self-signed cert");
        let cert_path = dir.path().join("tmp-cert.pem");
        let key_path = dir.path().join("tmp-key.pem");
        std::fs::write(&cert_path, cert.cert.pem().as_bytes()).unwrap();
        std::fs::write(&key_path, cert.signing_key.serialize_pem().as_bytes()).unwrap();

        let token_file = write_token_file(&format!("{}\n", TEST_TOKEN));

        let bind_admin = BindAdminSpec {
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 9000,
            tls: TlsTerminationSpec::Manual {
                cert: cert_path,
                key: key_path,
            },
            auth: valid_bearer_auth(token_file.path().to_path_buf()),
            ..Default::default()
        };
        let origin = Origin::test("bind_admin");

        // Act
        bind_admin.validate(&origin, &mut report);

        // Assert
        assert!(
            report.errors.is_empty(),
            "expected no errors; got: {:?}",
            report.errors
        );
    }
}
