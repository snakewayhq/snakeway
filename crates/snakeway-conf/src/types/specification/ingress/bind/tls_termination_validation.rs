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
