use crate::types::{DeviceSpec, IngressSpec, ServerSpec};
use crate::validation::{multi_file, single_file};
use confval::prelude::{Located, Report, Validate};

/// Validate everything that exists in a fully parsed config.
///
/// The version gate runs before any other rule. An unrecognized version means
/// the config targets a different schema, so every check below, including the
/// recorded field constraints that `validate_all` runs first, would validate
/// against the wrong rules. Emit only the version error and stop.
pub(crate) fn validate_spec(
    server: &ServerSpec,
    ingresses: &[Located<IngressSpec>],
    devices: &[Located<DeviceSpec>],
    report: &mut Report,
) {
    if server.version.value != 1 {
        report
            .error(format!("invalid config version: {}", server.version.value))
            .at(server.version.span)
            .help(
                "This version of Snakeway is not compatible with this config file. \
                 Please upgrade Snakeway.",
            )
            .emit();
        return;
    }

    server.validate_all(report);

    // Single file validation.
    single_file::validate_ingresses(ingresses, report);
    single_file::validate_devices(devices, report);

    // Multi-file validation.
    multi_file::validate_tls(server, ingresses, report);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BindSpec, IngressSpec, ServerSpec};
    use confval::prelude::Located;

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
    fn bad_version_suppresses_other_errors() {
        // The version gate is intentional: a bad version reports only the
        // version error and skips every v1 check, including the recorded
        // field constraints that `validate_all` would run first, even when a
        // field is otherwise invalid (here, threads far out of range). If
        // this ever reports two errors, the gate at the top of
        // `validate_spec` was moved or removed; that is a deliberate design
        // choice, not a bug to "fix".
        let mut report = Report::new();
        let server = ServerSpec {
            version: Located::detached(2),
            threads: Some(Located::detached(99_999)),
            ..Default::default()
        };

        validate_spec(&server, &[], &[], &mut report);

        assert_eq!(
            report.issues().len(),
            1,
            "expected only the version error, got: {:?}",
            report.issues()
        );
        assert!(
            report.issues()[0]
                .message
                .contains("invalid config version: 2")
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
