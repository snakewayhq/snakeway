use crate::validation::validator::validate_cert_key_pair;
use confval::hcl::{
    Fields, FromHcl, parse_string_field, parse_string_list_field, report_missing_field,
    report_unknown_field,
};
use confval::provenance::{Located, Report};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum TlsTerminationSpec {
    Manual {
        cert: Located<PathBuf>,
        key: Located<PathBuf>,
    },
    Acme {
        domains: Vec<Located<String>>,
        challenge: Located<String>,
    },
}

impl Default for TlsTerminationSpec {
    fn default() -> Self {
        TlsTerminationSpec::Manual {
            cert: Located::detached(PathBuf::new()),
            key: Located::detached(PathBuf::new()),
        }
    }
}

pub const ACME_CHALLENGE_HTTP01: &str = "http01";

/// The tls block carries a `mode` attribute selecting the variant,
/// mirroring the serialized form (`mode = "manual"` or `mode = "acme"`).
impl FromHcl for TlsTerminationSpec {
    fn from_hcl(fields: &Fields<'_>, report: &mut Report) -> Option<Self> {
        let Some(mode_field) = fields.get("mode") else {
            report_missing_field("mode", fields.enclosing(), report);
            return None;
        };
        let mode = parse_string_field(mode_field, report)?;

        match mode.value.as_str() {
            "manual" => {
                let mut cert = None;
                let mut key = None;
                for field in fields.iter() {
                    match field.name {
                        "mode" => {}
                        "cert" => {
                            cert = parse_string_field(field, report)
                                .map(|value| value.map(PathBuf::from));
                        }
                        "key" => {
                            key = parse_string_field(field, report)
                                .map(|value| value.map(PathBuf::from));
                        }
                        _ => report_unknown_field(field, report),
                    }
                }
                if cert.is_none() && !fields.has("cert") {
                    report_missing_field("cert", fields.enclosing(), report);
                }
                if key.is_none() && !fields.has("key") {
                    report_missing_field("key", fields.enclosing(), report);
                }
                Some(TlsTerminationSpec::Manual {
                    cert: cert?,
                    key: key?,
                })
            }
            "acme" => {
                let mut domains = None;
                let mut challenge = None;
                for field in fields.iter() {
                    match field.name {
                        "mode" => {}
                        "domains" => domains = parse_string_list_field(field, report),
                        "challenge" => challenge = parse_string_field(field, report),
                        _ => report_unknown_field(field, report),
                    }
                }
                if domains.is_none() && !fields.has("domains") {
                    report_missing_field("domains", fields.enclosing(), report);
                }
                Some(TlsTerminationSpec::Acme {
                    domains: domains?.value,
                    challenge: challenge
                        .unwrap_or_else(|| Located::detached(ACME_CHALLENGE_HTTP01.to_string())),
                })
            }
            other => {
                report
                    .error(format!("unknown tls mode: {other}"))
                    .at(mode.span)
                    .help("expected \"manual\" or \"acme\"")
                    .emit();
                None
            }
        }
    }
}

pub fn validate_tls_termination(spec: &TlsTerminationSpec, report: &mut Report) {
    match spec {
        TlsTerminationSpec::Manual { cert, key } => {
            if let Err(e) = validate_cert_key_pair(&cert.value, &key.value) {
                report
                    .error(format!("invalid TLS manual cert pair: {}", e))
                    .at(cert.span)
                    .help("Use manual mode instead")
                    .emit();
            }
        }
        TlsTerminationSpec::Acme { domains, challenge } => {
            if domains.is_empty() {
                report.error("missing domains for ACME TLS").emit();
            }
            if challenge.value != ACME_CHALLENGE_HTTP01 {
                report
                    .error(format!("unknown ACME challenge: {}", challenge.value))
                    .at(challenge.span)
                    .help("expected \"http01\"")
                    .emit();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::hcl::parse_hcl;
    use confval::provenance::SourceMap;

    use rcgen::generate_simple_self_signed;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    fn parse_tls(input: &str) -> (Report, Option<TlsTerminationSpec>) {
        let mut sources = SourceMap::new();
        let mut report = Report::new();
        let id = sources.add("ingress.hcl", input);
        let spec = parse_hcl::<TlsTerminationSpec>(&sources, id, &mut report);
        (report, spec)
    }

    #[test]
    fn parse_manual_mode() {
        // Arrange
        let input = "mode = \"manual\"\ncert = \"cert.pem\"\nkey = \"key.pem\"\n";

        // Act
        let (report, spec) = parse_tls(input);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        assert!(matches!(
            spec.unwrap(),
            TlsTerminationSpec::Manual { cert, .. } if cert.value == PathBuf::from("cert.pem")
        ));
    }

    #[test]
    fn parse_acme_mode_with_default_challenge() {
        // Arrange
        let input = "mode = \"acme\"\ndomains = [\"example.com\"]\n";

        // Act
        let (report, spec) = parse_tls(input);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let TlsTerminationSpec::Acme { domains, challenge } = spec.unwrap() else {
            panic!("expected acme");
        };
        assert_eq!(domains[0].value, "example.com");
        assert_eq!(challenge.value, ACME_CHALLENGE_HTTP01);
        assert!(challenge.span.is_detached());
    }

    #[test]
    fn parse_unknown_mode_is_reported() {
        // Arrange
        let input = "mode = \"automatic\"\n";

        // Act
        let (report, spec) = parse_tls(input);

        // Assert
        assert!(spec.is_none());
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "unknown tls mode: automatic")
        );
    }

    #[test]
    fn acme_tls_empty_domains_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = TlsTerminationSpec::Acme {
            domains: vec![],
            challenge: Located::detached(ACME_CHALLENGE_HTTP01.to_string()),
        };

        // Act
        validate_tls_termination(&spec, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.to_lowercase().contains("domain"))
        );
    }

    #[test]
    fn valid_acme_tls_with_domains() {
        // Arrange
        let mut report = Report::new();
        let spec = TlsTerminationSpec::Acme {
            domains: vec![Located::detached("example.com".to_string())],
            challenge: Located::detached(ACME_CHALLENGE_HTTP01.to_string()),
        };

        // Act
        validate_tls_termination(&spec, &mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn unknown_acme_challenge_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = TlsTerminationSpec::Acme {
            domains: vec![Located::detached("example.com".to_string())],
            challenge: Located::detached("dns01".to_string()),
        };

        // Act
        validate_tls_termination(&spec, &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message == "unknown ACME challenge: dns01")
        );
    }

    #[test]
    fn valid_manual_tls_with_real_certs() {
        // Arrange
        let dir = tempdir().expect("failed to create temp dir");
        let cert = generate_simple_self_signed(vec!["localhost".into()])
            .expect("failed to generate self-signed cert");
        let cert_pem = cert.cert.pem();
        let key_pem = cert.signing_key.serialize_pem();

        let cert_path = dir.path().join("cert.pem");
        let mut cert_file = File::create(&cert_path).expect("failed to create cert file");
        cert_file
            .write_all(cert_pem.as_bytes())
            .expect("failed to write cert");

        let key_path = dir.path().join("key.pem");
        let mut key_file = File::create(&key_path).expect("failed to create key file");
        key_file
            .write_all(key_pem.as_bytes())
            .expect("failed to write key");

        let mut report = Report::new();
        let spec = TlsTerminationSpec::Manual {
            cert: Located::detached(cert_path),
            key: Located::detached(key_path),
        };

        // Act
        validate_tls_termination(&spec, &mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn tls_missing_cert_and_key() {
        // Arrange
        let cert = PathBuf::from("/non/existent/cert.pem");
        let key = PathBuf::from("/non/existent/key.pem");
        let expected_error = format!(
            "invalid TLS manual cert pair: file does not exist: {}",
            cert.to_string_lossy()
        );
        let mut report = Report::new();
        let spec = TlsTerminationSpec::Manual {
            cert: Located::detached(cert),
            key: Located::detached(key),
        };

        // Act
        validate_tls_termination(&spec, &mut report);

        // Assert
        assert_eq!(report.issues()[0].message, expected_error);
    }
}
