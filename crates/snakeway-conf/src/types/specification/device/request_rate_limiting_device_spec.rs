use crate::validation::validator::validate_device_paths;
use confval::prelude::{Located, Report, Validate};
use confval::{RangeConstraint, range_constraint};
use serde::Serialize;

range_constraint!(MAX_REQUESTS_PER_SECOND, i64, min: 1, max: 30_000);
range_constraint!(WINDOW_SECONDS, i64, min: 1, max: 60, units: "seconds");

#[derive(Debug, Clone, Serialize, confval::Spec)]
pub struct RequestRateLimitingDeviceSpec {
    pub enable: Located<bool>,
    pub max_requests_per_second: Located<i64>,
    pub window_seconds: Located<i64>,

    /// Optional path prefixes this device applies to. Empty means all paths.
    #[confval(default)]
    pub paths: Vec<Located<String>>,
}

/// The default doubles as the `config init` template, so its values must pass
/// this spec's own validation. They match the documented example.
impl Default for RequestRateLimitingDeviceSpec {
    fn default() -> Self {
        Self {
            enable: Located::detached(false),
            max_requests_per_second: Located::detached(20),
            window_seconds: Located::detached(5),
            paths: Vec::new(),
        }
    }
}

impl Validate for RequestRateLimitingDeviceSpec {
    fn validate(&self, report: &mut Report) {
        MAX_REQUESTS_PER_SECOND.check_located(
            &self.max_requests_per_second,
            "max_requests_per_second",
            report,
        );
        WINDOW_SECONDS.check_located(&self.window_seconds, "window_seconds", report);

        validate_device_paths(&self.paths, report);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_spec_validates_clean() {
        // Arrange
        let mut report = Report::new();
        let spec = RequestRateLimitingDeviceSpec::default();

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    fn minimal() -> RequestRateLimitingDeviceSpec {
        RequestRateLimitingDeviceSpec {
            enable: Located::detached(true),
            max_requests_per_second: Located::detached(100),
            window_seconds: Located::detached(10),
            paths: vec![],
        }
    }

    #[test]
    fn valid_rate_limiting_device() {
        // Arrange
        let mut report = Report::new();
        let spec = minimal();

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    #[test]
    fn zero_rate_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = RequestRateLimitingDeviceSpec {
            max_requests_per_second: Located::detached(0),
            ..minimal()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(report.issues().iter().any(|e| {
            e.message
                .contains("max_requests_per_second must be at least 1")
        }));
    }

    #[test]
    fn zero_window_rejected() {
        // Arrange
        let mut report = Report::new();
        let spec = RequestRateLimitingDeviceSpec {
            window_seconds: Located::detached(0),
            ..minimal()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("window_seconds must be at least 1"))
        );
    }

    #[test]
    fn disabled_device_is_still_validated() {
        // Arrange
        let mut report = Report::new();
        let spec = RequestRateLimitingDeviceSpec {
            enable: Located::detached(false),
            max_requests_per_second: Located::detached(0),
            ..minimal()
        };

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report.has_issues(),
            "a disabled device must still validate its rate values"
        );
    }
}
