use confval::diagnostic::Report;
use confval::prelude::{Located, Validate};
use confval::{RangeConstraint, range_constraint};
use serde::Serialize;

range_constraint!(MAX_RETRIES, i64, min: 1, max: 60);

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct UpgradeSpec {
    /// Path to the Unix domain socket used for zero-drop upgrades.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sock: Option<Located<String>>,

    /// Maximum number of retries when connecting/accepting on the upgrade socket.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<Located<i64>>,
}

impl Validate for UpgradeSpec {
    fn validate(&self, report: &mut Report) {
        if let Some(sock) = &self.sock
            && sock.value.trim().is_empty()
        {
            report
                .error("upgrade.sock cannot be empty")
                .at(sock.span)
                .help("Provide a path to the Unix domain socket used for zero-drop upgrades.")
                .emit();
        }

        if let Some(retries) = &self.max_retries {
            MAX_RETRIES.check_located(retries, "max_retries", report);
        }
    }
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
        spec.validate(&mut report);

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
        spec.validate(&mut report);

        // Assert
        assert_eq!(report.issues().len(), 1);
        assert_eq!(report.issues()[0].message, "upgrade.sock cannot be empty");
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
        spec.validate(&mut report);

        // Assert
        assert_eq!(report.issues().len(), 1);
        assert_eq!(report.issues()[0].message, "max_retries must be at most 60");
    }
}
