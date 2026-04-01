use crate::types::{Origin, RequestRateLimitingDeviceSpec};
use crate::validation::{
    RangeConstraint, ValidateSpec, ValidationReport, range_constraint, validate_range_field,
};

range_constraint!(MAX_REQUESTS_PER_SECOND, u16, min: 1, max: 30_000);
range_constraint!(WINDOW_SECONDS, u16, min: 1, max: 60, units: "seconds");

impl ValidateSpec for RequestRateLimitingDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        validate_range_field!(
            MAX_REQUESTS_PER_SECOND,
            self.max_requests_per_second,
            report,
            origin
        );
        validate_range_field!(WINDOW_SECONDS, self.window_seconds, report, origin);

        for path in &self.paths {
            if !path.starts_with('/') {
                report.device_path_must_start_with_slash(path, origin);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::RequestRateLimitingDeviceSpec;
    use crate::validation::{ValidateSpec, ValidationReport};

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
        assert!(report.has_violations());
        assert!(
            report
                .errors
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
        assert!(report.has_violations());
        assert!(
            report
                .errors
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
        assert!(!report.has_violations());
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
        assert!(report.has_violations());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message.contains("must start with '/'"))
        );
    }
}
