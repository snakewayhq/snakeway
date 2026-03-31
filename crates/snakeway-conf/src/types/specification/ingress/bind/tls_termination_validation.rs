use crate::types::{Origin, TlsTerminationSpec};
use crate::validation::validator::validate_cert_key_pair;
use crate::validation::{ValidateSpec, ValidationReport};

impl ValidateSpec for TlsTerminationSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
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
    use crate::types::{Origin, TlsTerminationSpec};
    use crate::validation::{ValidateSpec, ValidationReport};
    use std::path::PathBuf;

    #[test]
    fn tls_missing_cert_and_key() {
        // Arrange
        let cert = PathBuf::from("/non/existent/cert.pem");
        let key = PathBuf::from("/non/existent/key.pem");
        let expected_error = format!(
            "invalid TLS manual cert pair: file does not exist: {}",
            cert.to_string_lossy()
        );
        let mut report = ValidationReport::default();
        let spec = TlsTerminationSpec::Manual { cert, key };
        let origin = Origin::test("tls");

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors[0].message, expected_error);
    }
}
