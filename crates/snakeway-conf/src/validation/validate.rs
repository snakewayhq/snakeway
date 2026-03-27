use crate::types::{DeviceSpec, IngressSpec, ServerSpec};
use crate::validation::report::ValidationReport;
use crate::validation::{ValidateSpec, multi_file, single_file};

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

    if server.version == 1 {
        report.invalid_config_version(&server.version, &server.origin);
        // Single file validation.
        server.validate(&server.origin, &mut report);
        single_file::validate_ingresses(ingresses, &mut report);
        single_file::validate_devices(devices, &mut report);

        // Multi file validation.
        multi_file::validate_tls(server, ingresses, &mut report);
    }

    report
}
