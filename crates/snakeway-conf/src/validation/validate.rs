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
        // Single file validation.
        server.validate(&server.origin, &mut report);
        single_file::validate_ingresses(ingresses, &mut report);
        single_file::validate_devices(devices, &mut report);

        // Multi file validation.
        multi_file::validate_tls(server, ingresses, &mut report);
    } else {
        report.invalid_config_version(&server.version, &server.origin);
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BindInterfaceInput, BindSpec, IngressSpec, ServerSpec};

    #[test]
    fn valid_version_runs_all_validation() {
        // Arrange
        let server = ServerSpec {
            version: 1,
            ..Default::default()
        };
        let ingress = IngressSpec {
            bind: Some(BindSpec {
                interface: BindInterfaceInput::Keyword("loopback".to_string()),
                port: 8080,
                ..Default::default()
            }),
            ..Default::default()
        };

        // Act
        let report = validate_spec(&server, &[ingress], &[]);

        // Assert
        assert!(
            report.errors.is_empty(),
            "expected no errors, got: {:?}",
            report.errors
        );
    }

    #[test]
    fn invalid_version_produces_error() {
        // Arrange
        let server = ServerSpec {
            version: 99,
            ..Default::default()
        };

        // Act
        let report = validate_spec(&server, &[], &[]);

        // Assert
        assert!(
            !report.errors.is_empty(),
            "expected at least one error for invalid version"
        );
        let has_version_error = report
            .errors
            .iter()
            .any(|e| e.message.contains("invalid config version"));
        assert!(
            has_version_error,
            "expected error about invalid config version, got: {:?}",
            report.errors
        );
    }
}
