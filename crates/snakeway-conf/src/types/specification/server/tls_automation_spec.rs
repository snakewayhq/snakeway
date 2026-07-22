use crate::validation::validator::{require_existing_dir, validate_cert_pem};
use confval::format::{
    Fields, FromFields, parse_string_field, report_missing_field, report_unknown_field,
};
use confval::prelude::{Located, Report, Validate};
use confval::{RangeConstraint, range_constraint};
use serde::Serialize;
use std::path::PathBuf;

range_constraint!(RENEW_WITHIN_DAYS, i64, min: 7, max: 30, units: "days");

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct TlsAutomationSpec {
    #[confval(nested)]
    pub acme: Located<AcmeServerSpec>,
    #[confval(nested)]
    pub cert_store: Located<CertStoreSpec>,
    #[confval(default = 30)]
    pub renew_within_days: Located<i64>,
}

#[derive(Debug, Serialize, Default)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CertStoreSpec {
    Filesystem {
        cert_dir: Located<PathBuf>,
    },
    #[default]
    Memory,
}

#[derive(Debug, Serialize, Default, confval::Spec)]
#[serde(rename_all = "lowercase")]
pub struct AcmeServerSpec {
    pub directory_url: Located<String>,
    pub data_dir: Located<PathBuf>,
    pub contact_email: Vec<Located<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_file: Option<Located<PathBuf>>,
}

impl Validate for AcmeServerSpec {
    fn validate(&self, report: &mut Report) {
        if self.directory_url.value.is_empty() {
            report
                .error("server TLS ACME directory URL cannot be empty")
                .at(self.directory_url.span)
                .emit();
        } else if !self.directory_url.value.starts_with("https://") {
            report
                .error("server TLS ACME directory URL must be a valid URL")
                .at(self.directory_url.span)
                .emit();
        }

        if self.contact_email.is_empty() {
            report
                .error("server TLS ACME contact email cannot be empty")
                .help("It must be a list of 1 or more email addresses")
                .emit();
        }

        if let Some(ca_file) = &self.ca_file
            && let Err(e) = validate_cert_pem(&ca_file.value)
        {
            report
                .error(format!(
                    "server TLS ACME CA file is invalid: {} - {}",
                    ca_file.value.to_string_lossy(),
                    e
                ))
                .at(ca_file.span)
                .help(
                    "In most production scenarios, this should not be set. \
                    For example, Let's Encrypt will use a root CA that is already \
                    trusted by your operating system. \
                    If you are using a custom CA in production or pebble for local development, you should \
                    set the server.tls.acme.ca_file option.",
                )
                .emit();
        }

        require_existing_dir(&self.data_dir, "server TLS ACME data_dir", report);
    }
}

/// The cert_store block carries a `type` attribute selecting the variant,
/// mirroring the serialized form (`type = "filesystem"` or `type = "memory"`).
impl FromFields for CertStoreSpec {
    fn from_fields(fields: &Fields, report: &mut Report) -> Option<Self> {
        let Some(type_field) = fields.get("type") else {
            report_missing_field("type", fields.enclosing(), report);
            return None;
        };
        let store_type = parse_string_field(type_field, report)?;

        match store_type.value.as_str() {
            "memory" => {
                for field in fields.iter() {
                    if field.name != "type" {
                        report_unknown_field(field, report);
                    }
                }
                Some(CertStoreSpec::Memory)
            }
            "filesystem" => {
                let mut cert_dir = None;
                for field in fields.iter() {
                    match field.name.as_str() {
                        "type" => {}
                        "cert_dir" => {
                            cert_dir = parse_string_field(field, report)
                                .map(|value| value.map(PathBuf::from));
                        }
                        _ => report_unknown_field(field, report),
                    }
                }
                if cert_dir.is_none() && !fields.has("cert_dir") {
                    report_missing_field("cert_dir", fields.enclosing(), report);
                }
                Some(CertStoreSpec::Filesystem {
                    cert_dir: cert_dir?,
                })
            }
            other => {
                report
                    .error(format!("unknown cert_store type: {other}"))
                    .at(store_type.span)
                    .help("expected \"memory\" or \"filesystem\"")
                    .emit();
                None
            }
        }
    }
}

impl Validate for TlsAutomationSpec {
    fn validate(&self, report: &mut Report) {
        self.acme.validate(report);

        RENEW_WITHIN_DAYS.check_located(&self.renew_within_days, "renew_within_days", report);

        validate_cert_store(&self.cert_store.value, report);
    }
}

fn validate_cert_store(spec: &CertStoreSpec, report: &mut Report) {
    match spec {
        CertStoreSpec::Filesystem { cert_dir } => {
            require_existing_dir(cert_dir, "server TLS filesystem cert_dir", report);
        }
        CertStoreSpec::Memory => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::format::hcl::parse_hcl;
    use confval::prelude::SourceMap;
    use std::path::Path;

    fn parse_tls(input: &str) -> (Report, Option<TlsAutomationSpec>) {
        let mut sources = SourceMap::new();
        let mut report = Report::new();
        let id = sources.add("snakeway.hcl", input);
        let spec = parse_hcl::<TlsAutomationSpec>(&sources, id, &mut report);
        (report, spec)
    }

    fn default_acme() -> AcmeServerSpec {
        AcmeServerSpec {
            directory_url: Located::detached(String::new()),
            data_dir: Located::detached(PathBuf::new()),
            contact_email: vec![],
            ca_file: None,
        }
    }

    #[test]
    fn parse_tls_automation_with_filesystem_store() {
        // Arrange
        let input = r#"renew_within_days = 14

acme {
  directory_url = "https://acme.example.com/dir"
  data_dir = "/tmp/acme"
  contact_email = ["admin@example.com"]
}

cert_store {
  type = "filesystem"
  cert_dir = "/etc/certs"
}
"#;

        // Act
        let (report, spec) = parse_tls(input);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let spec = spec.unwrap();
        assert_eq!(spec.renew_within_days.value, 14);
        assert_eq!(
            spec.acme.value.directory_url.value,
            "https://acme.example.com/dir"
        );
        assert!(matches!(
            &spec.cert_store.value,
            CertStoreSpec::Filesystem { cert_dir } if cert_dir.value == Path::new("/etc/certs")
        ));
    }

    #[test]
    fn parse_cert_store_memory() {
        // Arrange
        let input = r#"acme {
  directory_url = "https://acme.example.com/dir"
  data_dir = "/tmp/acme"
  contact_email = ["admin@example.com"]
}

cert_store {
  type = "memory"
}
"#;

        // Act
        let (report, spec) = parse_tls(input);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let spec = spec.unwrap();
        assert!(matches!(spec.cert_store.value, CertStoreSpec::Memory));
        assert_eq!(spec.renew_within_days.value, 30);
    }

    #[test]
    fn parse_cert_store_unknown_type() {
        // Arrange
        let input = r#"acme {
  directory_url = "https://acme.example.com/dir"
  data_dir = "/tmp/acme"
  contact_email = ["admin@example.com"]
}

cert_store {
  type = "redis"
}
"#;

        // Act
        let (report, spec) = parse_tls(input);

        // Assert: the failed nested child is reported, and the parent is still
        // constructed (with a default cert_store) so its siblings can validate.
        assert!(spec.is_some());
        assert!(matches!(
            spec.unwrap().cert_store.value,
            CertStoreSpec::Memory
        ));
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "unknown cert_store type: redis")
        );
    }

    #[test]
    fn parse_missing_acme_and_cert_store() {
        // Arrange
        let input = "renew_within_days = 14\n";

        // Act
        let (report, spec) = parse_tls(input);

        // Assert: both missing children are reported, and the parent still
        // constructs with defaults so the lowering gate (not a None parse)
        // is what blocks progress.
        assert!(spec.is_some());
        let messages: Vec<&str> = report.issues().iter().map(|i| i.message.as_str()).collect();
        assert!(messages.contains(&"missing required field: acme"));
        assert!(messages.contains(&"missing required field: cert_store"));
    }

    #[test]
    fn cert_store_failure_does_not_hide_sibling_semantic_error() {
        // Arrange: a structural failure on cert_store alongside a semantic
        // error on a sibling (renew_within_days below its minimum). Issue 9:
        // both must surface in one pass.
        let input = r#"renew_within_days = 3

acme {
  directory_url = "https://acme.example.com/dir"
  data_dir = "/tmp/acme"
  contact_email = ["admin@example.com"]
}

cert_store {
  type = "filesyste"
}
"#;

        // Act
        let (mut report, spec) = parse_tls(input);
        let spec = spec.expect("parent constructs despite the cert_store failure");
        spec.validate(&mut report);

        // Assert
        let messages: Vec<&str> = report.issues().iter().map(|i| i.message.as_str()).collect();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("unknown cert_store type")),
            "structural error missing: {messages:?}"
        );
        assert!(
            messages.contains(&"renew_within_days must be at least 7"),
            "sibling semantic error hidden: {messages:?}"
        );
    }

    #[test]
    fn acme_directory_url_cannot_be_empty() {
        // Arrange
        let mut report = Report::new();
        let spec = default_acme();

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message.contains("directory URL cannot be empty"))
        );
    }

    #[test]
    fn acme_directory_url_must_be_https() {
        // Arrange
        let mut report = Report::new();
        let spec = AcmeServerSpec {
            directory_url: Located::detached("http://example.com/acme".to_string()),
            ..default_acme()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "server TLS ACME directory URL must be a valid URL")
        );
    }

    #[test]
    fn acme_contact_email_cannot_be_empty() {
        // Arrange
        let mut report = Report::new();
        let spec = default_acme();

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "server TLS ACME contact email cannot be empty")
        );
    }

    #[test]
    fn cert_store_filesystem_missing_dir_is_invalid() {
        // Arrange
        let mut report = Report::new();
        let spec = CertStoreSpec::Filesystem {
            cert_dir: Located::detached(PathBuf::from("/non/existent/certs")),
        };

        // Act
        validate_cert_store(&spec, &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message.contains("cert_dir does not exist"))
        );
    }

    #[test]
    fn renew_within_days_out_of_range() {
        // Arrange
        let mut report = Report::new();
        let dir = tempfile::tempdir().unwrap();
        let spec = TlsAutomationSpec {
            acme: Located::detached(AcmeServerSpec {
                directory_url: Located::detached("https://acme.example.com/dir".to_string()),
                data_dir: Located::detached(dir.path().to_path_buf()),
                contact_email: vec![Located::detached("admin@example.com".to_string())],
                ca_file: None,
            }),
            cert_store: Located::detached(CertStoreSpec::Memory),
            renew_within_days: Located::detached(3),
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "renew_within_days must be at least 7")
        );
    }
}
