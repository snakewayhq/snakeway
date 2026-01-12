use crate::conf::types::{DeviceSpec, IngressSpec, ServerSpec};
use crate::conf::validation::report::ValidationReport;
use crate::conf::validation::single_file;

/// Validate everything that exists in a fully parsed config.
pub fn validate_spec(
    server: &ServerSpec,
    ingresses: &[IngressSpec],
    devices: &[DeviceSpec],
) -> ValidationReport {
    let mut report = ValidationReport {
        errors: vec![],
        warnings: vec![],
    };
    if single_file::validate_version(server, &mut report) {
        single_file::validate_server(server, &mut report);
        single_file::validate_ingresses(ingresses, &mut report);
        single_file::validate_devices(devices, &mut report);
    }
    report
}
