use crate::conf::types::{DeviceSpec, IngressSpec, ServerSpec};
use crate::conf::validation::report::ValidationReport;
use crate::conf::validation::{multi_file, single_file};

/// Validate everything that exists in a fully parsed config.
pub(crate) fn validate_spec(
    server: &ServerSpec,
    ingresses: &[IngressSpec],
    devices: &[DeviceSpec],
) -> ValidationReport {
    let mut report = ValidationReport {
        errors: vec![],
        warnings: vec![],
    };
    if single_file::validate_version(server, &mut report) {
        // Single file validation.
        single_file::validate_server(server, &mut report);
        single_file::validate_ingresses(ingresses, &mut report);
        single_file::validate_devices(devices, &mut report);

        // Multi file validation.
        multi_file::validate_tls(server, ingresses, &mut report);
    }
    report
}
