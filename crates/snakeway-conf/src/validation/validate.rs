use crate::types::{DeviceSpec, IngressSpec, ServerSpec, validate_server};
use crate::validation::{multi_file, single_file};
use confval::provenance::{Located, Report};

/// Validate everything that exists in a fully parsed config.
pub(crate) fn validate_spec(
    server: &ServerSpec,
    ingresses: &[Located<IngressSpec>],
    devices: &[Located<DeviceSpec>],
    report: &mut Report,
) {
    validate_server(server, report);

    if server.version.value == 1 {
        // Single file validation.
        single_file::validate_ingresses(ingresses, report);
        single_file::validate_devices(devices, report);

        // Multi-file validation.
        multi_file::validate_tls(server, ingresses, report);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BindSpec, IngressSpec, ServerSpec};
    use confval::provenance::Located;

    #[test]
    fn valid_version_runs_all_validation() {
        // Arrange
        let server = ServerSpec::default();
        let ingress = Located::detached(IngressSpec {
            bind: Some(Located::detached(BindSpec {
                interface: Located::detached("loopback".to_string()),
                port: Located::detached(8080),
                ..Default::default()
            })),
            ..Default::default()
        });
        let mut report = Report::new();

        // Act
        validate_spec(&server, &[ingress], &[], &mut report);

        // Assert
        assert!(
            !report.has_errors(),
            "expected no errors, got: {:?}",
            report.issues()
        );
    }

    #[test]
    fn invalid_version_produces_error() {
        // Arrange
        let server = ServerSpec {
            version: Located::detached(99),
            ..Default::default()
        };
        let mut report = Report::new();

        // Act
        validate_spec(&server, &[], &[], &mut report);

        // Assert
        let has_version_error = report
            .issues()
            .iter()
            .any(|i| i.message.contains("invalid config version"));
        assert!(
            has_version_error,
            "expected error about invalid config version, got: {:?}",
            report.issues()
        );
    }
}
