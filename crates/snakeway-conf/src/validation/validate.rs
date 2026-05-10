use crate::types::{DeviceSpec, HclOrigin, IngressSpec, ServerSpec};
use crate::validation::{multi_file, single_file};
use confval::{ValidateSpec, ValidationIssue, ValidationReport};

/// Validate everything that exists in a fully parsed config.
pub(crate) fn validate_spec(
    server: &ServerSpec,
    ingresses: &[IngressSpec],
    devices: &[DeviceSpec],
) -> ValidationReport<HclOrigin> {
    let mut report = ValidationReport::default();

    if server.version == 1 {
        // Single file validation.
        server.validate(&server.origin, &mut report);
        single_file::validate_ingresses(ingresses, &mut report);
        single_file::validate_devices(devices, &mut report);

        // Multi-file validation.
        multi_file::validate_tls(server, ingresses, &mut report);
    } else {
        report.error(
            ValidationIssue::error_with_help(
                format!("invalid config version: {}", &server.version),
                server.origin.clone(),
                "This version of Snakeway is not compatible with this config file. Please upgrade Snakeway."
            )
        );
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
            report.errors().is_empty(),
            "expected no errors, got: {:?}",
            report.errors()
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
            !report.errors().is_empty(),
            "expected at least one error for invalid version"
        );
        let has_version_error = report
            .errors()
            .iter()
            .any(|e| e.message.contains("invalid config version"));
        assert!(
            has_version_error,
            "expected error about invalid config version, got: {:?}",
            report.errors()
        );
    }
}
