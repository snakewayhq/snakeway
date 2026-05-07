use crate::types::{CircuitBreakerSpec, OriginDeprecated};
use crate::validation::{
    RangeConstraint, ValidateSpec, ValidationReportDeprecated, range_constraint,
    validate_range_field,
};

range_constraint!(FAILURE_THRESHOLD, u32, min: 1, max: 10_000);
range_constraint!(OPEN_DURATION_MS, u64, min: 1, max: 60 * 60 * 1000, units: "ms");
range_constraint!(HALF_OPEN_MAX_REQUESTS, u32, min: 1, max: 10_000);
range_constraint!(SUCCESS_THRESHOLD, u32, min: 1, max: 10_000);

impl ValidateSpec for CircuitBreakerSpec {
    fn validate(&self, origin: &OriginDeprecated, report: &mut ValidationReportDeprecated) {
        validate_range_field!(FAILURE_THRESHOLD, self.failure_threshold, report, origin);
        validate_range_field!(
            OPEN_DURATION_MS,
            self.open_duration_milliseconds,
            report,
            origin
        );
        validate_range_field!(
            HALF_OPEN_MAX_REQUESTS,
            self.half_open_max_requests,
            report,
            origin
        );
        validate_range_field!(SUCCESS_THRESHOLD, self.success_threshold, report, origin);
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{CircuitBreakerSpec, OriginDeprecated};
    use crate::validation::{ValidateSpec, ValidationReportDeprecated};

    #[test]
    fn valid_circuit_breaker() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let spec = CircuitBreakerSpec {
            enable_auto_recovery: true,
            failure_threshold: 5,
            open_duration_milliseconds: 1000,
            half_open_max_requests: 1,
            success_threshold: 2,
            ..Default::default()
        };
        let origin = OriginDeprecated::test("circuit_breaker");

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(!report.has_violations());
    }

    #[test]
    fn failure_threshold_out_of_range() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let spec = CircuitBreakerSpec {
            enable_auto_recovery: true,
            failure_threshold: 0,
            open_duration_milliseconds: 1000,
            half_open_max_requests: 1,
            success_threshold: 2,
            ..Default::default()
        };
        let origin = OriginDeprecated::test("circuit_breaker");

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        let error = report.errors.first().expect("expected at least one error");
        assert!(error.message.contains("failure_threshold"));
    }

    #[test]
    fn open_duration_out_of_range() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let spec = CircuitBreakerSpec {
            enable_auto_recovery: true,
            failure_threshold: 5,
            open_duration_milliseconds: 0,
            half_open_max_requests: 1,
            success_threshold: 2,
            ..Default::default()
        };
        let origin = OriginDeprecated::test("circuit_breaker");

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        let error = report.errors.first().expect("expected at least one error");
        assert!(error.message.contains("open_duration_milliseconds"));
    }

    #[test]
    fn half_open_max_requests_out_of_range() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let spec = CircuitBreakerSpec {
            enable_auto_recovery: true,
            failure_threshold: 5,
            open_duration_milliseconds: 1000,
            half_open_max_requests: 10001,
            success_threshold: 2,
            ..Default::default()
        };
        let origin = OriginDeprecated::test("circuit_breaker");

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        let error = report.errors.first().expect("expected at least one error");
        assert!(error.message.contains("half_open_max_requests"));
    }

    #[test]
    fn success_threshold_out_of_range() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let spec = CircuitBreakerSpec {
            enable_auto_recovery: true,
            failure_threshold: 5,
            open_duration_milliseconds: 1000,
            half_open_max_requests: 1,
            success_threshold: 0,
            ..Default::default()
        };
        let origin = OriginDeprecated::test("circuit_breaker");

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        let error = report.errors.first().expect("expected at least one error");
        assert!(error.message.contains("success_threshold"));
    }
}
