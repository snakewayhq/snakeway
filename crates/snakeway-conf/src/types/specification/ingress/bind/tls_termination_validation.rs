use crate::types::{OriginDeprecated, TlsTerminationSpec};
use crate::validation::validator::validate_cert_key_pair;
use crate::validation::{ValidateSpec, ValidationReportDeprecated};

impl ValidateSpec for TlsTerminationSpec {
    fn validate(&self, origin: &OriginDeprecated, report: &mut ValidationReportDeprecated) {
        match self {
            TlsTerminationSpec::Manual { cert, key } => {
                if let Err(e) = validate_cert_key_pair(cert, key) {
                    report.ingress_tls_manual_cert_pair_invalid(&e, origin);
                }
            }
            TlsTerminationSpec::Acme { domains, .. } => {
                if domains.is_empty() {
                    report.acme_tls_requires_domains(origin);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{OriginDeprecated, TlsTerminationSpec};
    use crate::validation::{ValidateSpec, ValidationReportDeprecated};
    use std::path::PathBuf;

    use rcgen::generate_simple_self_signed;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn acme_tls_empty_domains_rejected() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let spec = TlsTerminationSpec::Acme {
            domains: vec![],
            challenge: Default::default(),
        };
        let origin = OriginDeprecated::test("tls");

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.to_lowercase().contains("domain"))
        );
    }

    #[test]
    fn valid_acme_tls_with_domains() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let spec = TlsTerminationSpec::Acme {
            domains: vec!["example.com".to_string()],
            challenge: Default::default(),
        };
        let origin = OriginDeprecated::test("tls");

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(!report.has_violations());
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

        let mut report = ValidationReportDeprecated::default();
        let spec = TlsTerminationSpec::Manual {
            cert: cert_path,
            key: key_path,
        };
        let origin = OriginDeprecated::test("tls");

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(!report.has_violations());
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
        let mut report = ValidationReportDeprecated::default();
        let spec = TlsTerminationSpec::Manual { cert, key };
        let origin = OriginDeprecated::test("tls");

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors[0].message, expected_error);
    }
}
