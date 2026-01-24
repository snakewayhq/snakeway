use crate::conf::types::Origin;
use crate::conf::validation::ValidationReport;
use http::{HeaderName, Method};

pub fn validate_http_header_name(header: &str, report: &mut ValidationReport, origin: &Origin) {
    if HeaderName::from_bytes(header.as_bytes()).is_err() {
        report.invalid_http_header_name(header, origin);
    }

    if header.as_bytes().iter().all(|b| !b.is_ascii_uppercase()) {
        report.invalid_http_header_name(header, origin);
    }
}

pub fn validate_http_method(method: &str, report: &mut ValidationReport, origin: &Origin) {
    if Method::from_bytes(method.as_bytes()).is_err() {
        report.invalid_http_method(method, origin);
    }
}
