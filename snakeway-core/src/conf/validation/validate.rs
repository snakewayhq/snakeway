use crate::conf::types::{DeviceConfig, IngressSpec, ServerSpec};
use crate::conf::validation::report::ValidationReport;
use crate::conf::validation::validator;

/// Validate everything that exists in a fully parsed config.
pub fn validate_spec(
    server: &ServerSpec,
    ingresses: &[IngressSpec],
    devices: &[DeviceConfig],
) -> ValidationReport {
    let mut report = ValidationReport {
        errors: vec![],
        warnings: vec![],
    };
    if validator::validate_version(server, &mut report) {
        validator::validate_server(server, &mut report);
        validator::validate_ingresses(ingresses, &mut report);
        validator::validate_devices(devices, &mut report);
    }
    report
}
