use crate::types::HclInt;
use crate::validation::validator::validate_device_paths;
use confval::provenance::{Located, Report};
use confval::{RangeConstraint, range_constraint};
use serde::Serialize;

range_constraint!(MAX_REQUESTS_PER_SECOND, i64, min: 1, max: 30_000);
range_constraint!(WINDOW_SECONDS, i64, min: 1, max: 60, units: "seconds");

#[derive(Debug, Clone, Default, Serialize, confval::Spec)]
pub struct RequestRateLimitingDeviceSpec {
    pub enable: Located<bool>,
    pub max_requests_per_second: Located<HclInt>,
    pub window_seconds: Located<HclInt>,

    /// Optional path prefixes this device applies to. Empty means all paths.
    #[confval(default)]
    pub paths: Vec<Located<String>>,
}

pub fn validate_request_rate_limiting_device(
    spec: &RequestRateLimitingDeviceSpec,
    report: &mut Report,
) {
    MAX_REQUESTS_PER_SECOND.check_located(
        &spec.max_requests_per_second,
        "max_requests_per_second",
        report,
    );
    WINDOW_SECONDS.check_located(&spec.window_seconds, "window_seconds", report);

    validate_device_paths(&spec.paths, report);
}

#[cfg(test)]
mod tests {
    use super::*;

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
        validate_request_rate_limiting_device(&spec, &mut report);

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
        validate_request_rate_limiting_device(&spec, &mut report);

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
        validate_request_rate_limiting_device(&spec, &mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("window_seconds must be at least 1"))
        );
    }
}
