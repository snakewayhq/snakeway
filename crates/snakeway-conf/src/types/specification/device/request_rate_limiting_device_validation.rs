use crate::types::{HclOrigin, RequestRateLimitingDeviceSpec};
use crate::validation::validator::validate_device_paths;
use confval::{
    RangeConstraint, ValidateSpec, ValidationReport, range_constraint, validate_range_field,
};

range_constraint!(MAX_REQUESTS_PER_SECOND, i64, min: 1, max: 30_000);
range_constraint!(WINDOW_SECONDS, i64, min: 1, max: 60, units: "seconds");

impl ValidateSpec<HclOrigin> for RequestRateLimitingDeviceSpec {
    fn validate(&self, origin: &HclOrigin, report: &mut ValidationReport<HclOrigin>) {
        validate_range_field!(
            MAX_REQUESTS_PER_SECOND,
            self.max_requests_per_second,
            report,
            origin
        );
        validate_range_field!(WINDOW_SECONDS, self.window_seconds, report, origin);

        validate_device_paths(&self.paths, report, origin);
    }
}

#[cfg(test)]
mod tests {
    use crate::types::RequestRateLimitingDeviceSpec;
    use confval::{ValidateSpec, ValidationReport};

    #[test]
    fn max_requests_per_second_below_range() {
        // Arrange
        let mut report = ValidationReport::default();
        let spec = RequestRateLimitingDeviceSpec {
            enable: true,
            max_requests_per_second: 0,
            window_seconds: 10,
            ..Default::default()
        };

        // Act
        spec.validate(&spec.origin, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .errors()
                .iter()
                .any(|e| e.message.contains("max_requests_per_second"))
        );
    }

    #[test]
    fn window_seconds_below_range() {
        // Arrange
        let mut report = ValidationReport::default();
        let spec = RequestRateLimitingDeviceSpec {
            enable: true,
            max_requests_per_second: 100,
            window_seconds: 0,
            ..Default::default()
        };

        // Act
        spec.validate(&spec.origin, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .errors()
                .iter()
                .any(|e| e.message.contains("window_seconds"))
        );
    }

    #[test]
    fn valid_rate_limiting_device() {
        // Arrange
        let mut report = ValidationReport::default();
        let spec = RequestRateLimitingDeviceSpec {
            enable: true,
            max_requests_per_second: 100,
            window_seconds: 10,
            ..Default::default()
        };

        // Act
        spec.validate(&spec.origin, &mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn path_without_leading_slash_is_invalid() {
        // Arrange
        let mut report = ValidationReport::default();
        let spec = RequestRateLimitingDeviceSpec {
            enable: true,
            max_requests_per_second: 100,
            window_seconds: 10,
            paths: vec!["api/v1".to_string()],
            ..Default::default()
        };

        // Act
        spec.validate(&spec.origin, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .errors()
                .iter()
                .any(|e| e.message.contains("must start with '/'"))
        );
    }
}
