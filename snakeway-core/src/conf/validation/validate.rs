use crate::conf::types::{DeviceConfig, ExposeServerConfig, IngressConfig};
use crate::conf::validation::report::ValidationReport;
use crate::conf::validation::validator;

/// Validate everything that exists in a fully parsed config.
pub fn validate_dsl_config(
    server: &ExposeServerConfig,
    ingresses: &[IngressConfig],
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
