use crate::types::{AcmeServerSpec, CertStoreSpec, Origin, TlsAutomationSpec};
use crate::validation::validator::validate_cert_pem;
use crate::validation::{RangeConstraint, ValidateSpec, validate_range_field};
use crate::validation::{ValidationReport, range_constraint};
use nix::NixPath;

range_constraint!(RENEW_WITHIN_DAYS, u64, min: 7, max: 30, units: "days");

impl ValidateSpec for TlsAutomationSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        self.acme.validate(origin, report);

        validate_range_field!(RENEW_WITHIN_DAYS, self.renew_within_days, report, origin);

        self.cert_store.validate(origin, report);
    }
}

impl ValidateSpec for AcmeServerSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if self.directory_url.is_empty() {
            report.server_tls_acme_directory_url_cannot_be_empty(origin);
        } else if !self.directory_url.starts_with("https://") {
            report.server_tls_acme_directory_url_must_be_https(origin);
        }

        if self.contact_email.is_empty() {
            report.server_tls_acme_contact_email_cannot_be_empty(origin);
        }

        if let Some(ca_file) = &self.ca_file
            && let Err(e) = validate_cert_pem(ca_file)
        {
            report.server_tls_acme_ca_file_invalid(ca_file, &e, origin);
        }

        if self.data_dir.is_empty() {
            report.server_tls_acme_data_dir_cannot_be_empty(origin);
        } else if !self.data_dir.is_dir() {
            report.server_tls_acme_data_dir_is_invalid(&self.data_dir, origin);
        }
    }
}

impl ValidateSpec for CertStoreSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        match self {
            CertStoreSpec::Filesystem { cert_dir } => {
                if cert_dir.is_empty() {
                    report.server_tls_cert_dir_cannot_be_empty(origin);
                } else if !cert_dir.is_dir() {
                    report.server_tls_cert_dir_is_invalid(cert_dir, origin);
                }
            }
            CertStoreSpec::Memory => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{AcmeServerSpec, CertStoreSpec, Origin, TlsAutomationSpec};
    use crate::validation::{ValidateSpec, ValidationReport};
    use std::path::PathBuf;

    fn default_acme() -> AcmeServerSpec {
        AcmeServerSpec {
            directory_url: String::new(),
            data_dir: PathBuf::new(),
            contact_email: vec![],
            ca_file: None,
        }
    }

    #[test]
    fn acme_directory_url_cannot_be_empty() {
        // Arrange
        let origin = Origin::test("tls_automation");
        let mut report = ValidationReport::default();
        let spec = AcmeServerSpec {
            directory_url: String::new(),
            ..default_acme()
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("directory_url")
                    || e.message.contains("cannot be empty"))
        );
    }

    #[test]
    fn acme_directory_url_must_be_https() {
        // Arrange
        let origin = Origin::test("tls_automation");
        let mut report = ValidationReport::default();
        let spec = AcmeServerSpec {
            directory_url: "http://example.com/acme".to_string(),
            ..default_acme()
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("directory URL must be a valid URL"))
        );
    }

    #[test]
    fn acme_contact_email_cannot_be_empty() {
        // Arrange
        let origin = Origin::test("tls_automation");
        let mut report = ValidationReport::default();
        let spec = AcmeServerSpec {
            directory_url: "https://acme.example.com/directory".to_string(),
            contact_email: vec![],
            ..default_acme()
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("contact email cannot be empty"))
        );
    }

    #[test]
    fn acme_ca_file_invalid() {
        // Arrange
        let origin = Origin::test("tls_automation");
        let mut report = ValidationReport::default();
        let spec = AcmeServerSpec {
            directory_url: "https://acme.example.com/directory".to_string(),
            ca_file: Some(PathBuf::from("/non/existent/ca.pem")),
            ..default_acme()
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("ca_file") || e.message.contains("ca.pem"))
        );
    }

    #[test]
    fn acme_data_dir_cannot_be_empty() {
        // Arrange
        let origin = Origin::test("tls_automation");
        let mut report = ValidationReport::default();
        let spec = AcmeServerSpec {
            directory_url: "https://acme.example.com/directory".to_string(),
            contact_email: vec!["admin@example.com".to_string()],
            data_dir: PathBuf::new(),
            ca_file: None,
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("data_dir") && e.message.contains("path is required"))
        );
    }

    #[test]
    fn acme_data_dir_must_be_a_directory() {
        // Arrange
        let origin = Origin::test("tls_automation");
        let mut report = ValidationReport::default();
        let spec = AcmeServerSpec {
            directory_url: "https://acme.example.com/directory".to_string(),
            contact_email: vec!["admin@example.com".to_string()],
            data_dir: PathBuf::from("/non/existent/data_dir"),
            ca_file: None,
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(report.errors.iter().any(|e| e.message.contains("data_dir")));
    }

    #[test]
    fn cert_dir_cannot_be_empty() {
        // Arrange
        let origin = Origin::test("tls_automation");
        let mut report = ValidationReport::default();
        let spec = CertStoreSpec::Filesystem {
            cert_dir: PathBuf::new(),
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("cert_dir") && e.message.contains("path is required"))
        );
    }

    #[test]
    fn cert_dir_must_be_a_directory() {
        // Arrange
        let origin = Origin::test("tls_automation");
        let mut report = ValidationReport::default();
        let spec = CertStoreSpec::Filesystem {
            cert_dir: PathBuf::from("/non/existent/cert_dir"),
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(report.errors.iter().any(|e| e.message.contains("cert_dir")));
    }

    #[test]
    fn renew_within_days_below_range() {
        // Arrange
        let origin = Origin::test("tls_automation");
        let mut report = ValidationReport::default();
        let dir = tempfile::tempdir().unwrap();
        let spec = TlsAutomationSpec {
            acme: AcmeServerSpec {
                directory_url: "https://acme.example.com/directory".to_string(),
                contact_email: vec!["admin@example.com".to_string()],
                data_dir: dir.path().to_path_buf(),
                ca_file: None,
            },
            cert_store: CertStoreSpec::Memory,
            renew_within_days: 6,
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("renew_within_days"))
        );
    }

    #[test]
    fn renew_within_days_above_range() {
        // Arrange
        let origin = Origin::test("tls_automation");
        let mut report = ValidationReport::default();
        let dir = tempfile::tempdir().unwrap();
        let spec = TlsAutomationSpec {
            acme: AcmeServerSpec {
                directory_url: "https://acme.example.com/directory".to_string(),
                contact_email: vec!["admin@example.com".to_string()],
                data_dir: dir.path().to_path_buf(),
                ca_file: None,
            },
            cert_store: CertStoreSpec::Memory,
            renew_within_days: 31,
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("renew_within_days"))
        );
    }

    #[test]
    fn valid_acme_server() {
        // Arrange
        let origin = Origin::test("tls_automation");
        let mut report = ValidationReport::default();
        let dir = tempfile::tempdir().unwrap();
        let spec = AcmeServerSpec {
            directory_url: "https://acme.example.com/directory".to_string(),
            contact_email: vec!["admin@example.com".to_string()],
            data_dir: dir.path().to_path_buf(),
            ca_file: None,
        };

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(!report.has_violations());
    }
}
