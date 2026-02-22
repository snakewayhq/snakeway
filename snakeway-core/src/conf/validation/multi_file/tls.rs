use crate::conf::types::{IngressSpec, ServerSpec};
use crate::conf::validation::ValidationReport;

pub fn validate_tls(server: &ServerSpec, ingresses: &[IngressSpec], report: &mut ValidationReport) {
    // Validate TLS configuration for server and ingresses.
    // todo ...
}
