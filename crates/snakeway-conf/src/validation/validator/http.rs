use crate::types::Origin;
use crate::validation::ValidationReport;
use http::{HeaderName, Method};

pub(crate) fn validate_http_header_name(
    header: &str,
    report: &mut ValidationReport,
    origin: &Origin,
) {
    if HeaderName::from_bytes(header.as_bytes()).is_err() {
        report.invalid_http_header_name(header, origin);
    }
}

pub(crate) fn validate_http_method(method: &str, report: &mut ValidationReport, origin: &Origin) {
    if Method::from_bytes(method.as_bytes()).is_err() {
        report.invalid_http_method(method, origin);
    }
}
