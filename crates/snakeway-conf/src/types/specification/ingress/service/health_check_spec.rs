use confval::prelude::{Located, Report, Validate, range_constraint};
use serde::Serialize;

range_constraint!(FAILURE_THRESHOLD, i64, min: 1, max: 10_000);
range_constraint!(UNHEALTHY_COOLDOWN_SECONDS, i64, min: 1, max: 3600, units: "seconds");

#[derive(Debug, Clone, Serialize, confval::Spec)]
pub struct HealthCheckSpec {
    pub enable: Located<bool>,
    #[confval(default = 3, range = FAILURE_THRESHOLD)]
    pub failure_threshold: Located<i64>,
    #[confval(default = 10, range = UNHEALTHY_COOLDOWN_SECONDS)]
    pub unhealthy_cooldown_seconds: Located<i64>,
}

impl Default for HealthCheckSpec {
    fn default() -> Self {
        Self {
            enable: Located::detached(false),
            failure_threshold: Located::detached(3),
            unhealthy_cooldown_seconds: Located::detached(10),
        }
    }
}

impl Validate for HealthCheckSpec {
    fn validate(&self, _report: &mut Report) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_defaults_are_valid() {
        // Arrange
        let mut report = Report::new();
        let spec = HealthCheckSpec {
            enable: Located::detached(true),
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    #[test]
    fn disabled_health_check_still_checks_ranges() {
        // Arrange
        let mut report = Report::new();
        let spec = HealthCheckSpec {
            enable: Located::detached(false),
            failure_threshold: Located::detached(0),
            unhealthy_cooldown_seconds: Located::detached(0),
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("failure_threshold")),
            "a disabled health check must still validate its values; issues: {:?}",
            report.issues()
        );
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("unhealthy_cooldown_seconds")),
            "a disabled health check must still validate its values; issues: {:?}",
            report.issues()
        );
    }

    #[test]
    fn failure_threshold_below_minimum_is_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = HealthCheckSpec {
            enable: Located::detached(true),
            failure_threshold: Located::detached(0),
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert_eq!(report.issues().len(), 1);
        assert_eq!(
            report.issues()[0].message,
            "failure_threshold must be at least 1"
        );
    }

    #[test]
    fn unhealthy_cooldown_above_maximum_is_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = HealthCheckSpec {
            enable: Located::detached(true),
            unhealthy_cooldown_seconds: Located::detached(3601),
            ..Default::default()
        };

        // Act
        spec.validate_all(&mut report);

        // Assert
        assert_eq!(report.issues().len(), 1);
        assert_eq!(
            report.issues()[0].message,
            "unhealthy_cooldown_seconds must be at most 3600"
        );
    }
}
