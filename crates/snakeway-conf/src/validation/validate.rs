use crate::types::{DeviceSpec, HclOrigin, IngressSpec, ServerSpec, validate_server};
use crate::validation::{multi_file, single_file};
use confval::ValidationReport;
use confval::provenance::{Located, Report};

/// Validate everything that exists in a fully parsed config.
///
/// All issues go to the span-first `span_report`. The origin-based report
/// is an empty vestige kept only until the 0.1 API is deleted.
pub(crate) fn validate_spec(
    server: &ServerSpec,
    ingresses: &[Located<IngressSpec>],
    devices: &[Located<DeviceSpec>],
    span_report: &mut Report,
) -> ValidationReport<HclOrigin> {
    let report = ValidationReport::default();

    validate_server(server, span_report);

    if server.version.value == 1 {
        // Single file validation.
        single_file::validate_ingresses(ingresses, span_report);
        single_file::validate_devices(devices, span_report);

        // Multi-file validation.
        multi_file::validate_tls(server, ingresses, span_report);
    }

    report
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
        let mut span_report = Report::new();

        // Act
        let report = validate_spec(&server, &[ingress], &[], &mut span_report);

        // Assert
        assert!(
            report.errors().is_empty(),
            "expected no errors, got: {:?}",
            report.errors()
        );
        assert!(
            !span_report.has_errors(),
            "expected no span errors, got: {:?}",
            span_report.issues()
        );
    }

    #[test]
    fn invalid_version_produces_error() {
        // Arrange
        let server = ServerSpec {
            version: Located::detached(99),
            ..Default::default()
        };
        let mut span_report = Report::new();

        // Act
        let report = validate_spec(&server, &[], &[], &mut span_report);

        // Assert
        assert!(report.errors().is_empty());
        let has_version_error = span_report
            .issues()
            .iter()
            .any(|i| i.message.contains("invalid config version"));
        assert!(
            has_version_error,
            "expected error about invalid config version, got: {:?}",
            span_report.issues()
        );
    }
}
