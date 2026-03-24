use crate::types::ServerSpec;
use crate::validation::report::ValidationReport;
use crate::validation::validate_spec_trait::ValidateSpec;

/// Validate top-level config version.
///
/// Fail-fast: invalid versions invalidate the entire config model.
pub(crate) fn validate_version(server_spec: &ServerSpec, report: &mut ValidationReport) -> bool {
    if server_spec.version != 1 {
        report.invalid_config_version(&server_spec.version, &server_spec.origin);
        return false;
    }
    true
}

/// Validate the server config.
///
/// Delegates field-local validation to the `ValidateSpec` trait implementation
/// on `ServerSpec`. Cross-field and cross-file checks (if any) remain here.
pub(crate) fn validate_server(server_spec: &ServerSpec, report: &mut ValidationReport) {
    server_spec.validate(&server_spec.origin, report);
}
