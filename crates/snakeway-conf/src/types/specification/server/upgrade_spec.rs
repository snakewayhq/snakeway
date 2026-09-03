use confval::diagnostic::Report;
use confval::prelude::{Located, Validate, range_constraint};
use serde::Serialize;

range_constraint!(MAX_RETRIES, i64, min: 1, max: 60);

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct UpgradeSpec {
    /// Path to the Unix domain socket used for zero-drop upgrades.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(non_empty(
        help = "Provide a path to the Unix domain socket used for zero-drop upgrades."
    ))]
    pub sock: Option<Located<String>>,

    /// Maximum number of retries when connecting/accepting on the upgrade socket.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(range = MAX_RETRIES)]
    pub max_retries: Option<Located<i64>>,
}

impl Validate for UpgradeSpec {
    fn validate(&self, _report: &mut Report) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_fields_are_valid() {
        // Arrange
        let mut report = Report::new();
        let spec = UpgradeSpec::default();

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    #[test]
    fn blank_sock_is_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = UpgradeSpec {
            sock: Some(Located::detached("   ".to_string())),
            max_retries: None,
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert_eq!(report.issues().len(), 1);
        assert_eq!(report.issues()[0].message, "sock must not be empty");
        assert_eq!(
            report.issues()[0].help.as_deref(),
            Some("Provide a path to the Unix domain socket used for zero-drop upgrades.")
        );
    }

    #[test]
    fn max_retries_above_maximum_is_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = UpgradeSpec {
            sock: Some(Located::detached("/tmp/upgrade.sock".to_string())),
            max_retries: Some(Located::detached(61)),
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert_eq!(report.issues().len(), 1);
        assert_eq!(report.issues()[0].message, "max_retries must be at most 60");
    }
}
