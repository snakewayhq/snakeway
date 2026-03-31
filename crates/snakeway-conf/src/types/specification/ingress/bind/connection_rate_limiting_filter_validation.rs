use crate::types::{ConnectionRateLimitingFilterSpec, Origin};
use crate::validation::{
    RangeConstraint, ValidateSpec, ValidationReport, range_constraint, validate_range_field,
};

range_constraint!(REACTION_INTERVAL_IN_SECONDS, u16, min: 1, max: 60, units: "seconds");
range_constraint!(MAX_CONNECTIONS_PER_SECOND, u16, min: 1, max: 30_000);

impl ValidateSpec for ConnectionRateLimitingFilterSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        validate_range_field!(
            REACTION_INTERVAL_IN_SECONDS,
            self.window_seconds,
            report,
            origin
        );
        validate_range_field!(
            MAX_CONNECTIONS_PER_SECOND,
            self.max_connections_per_second,
            report,
            origin
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{ConnectionRateLimitingFilterSpec, Origin};
    use crate::validation::{ValidateSpec, ValidationReport};

    fn test_origin() -> Origin {
        Origin::test("connection_rate_limiting_filter")
    }

    #[test]
    fn window_seconds_below_range() {
        // Arrange
        let spec = ConnectionRateLimitingFilterSpec {
            window_seconds: 0,
            max_connections_per_second: 100,
        };
        let origin = test_origin();
        let mut report = ValidationReport::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors.len(), 1);
        assert!(report.errors[0].message.contains("window_seconds"));
    }

    #[test]
    fn max_connections_below_range() {
        // Arrange
        let spec = ConnectionRateLimitingFilterSpec {
            window_seconds: 10,
            max_connections_per_second: 0,
        };
        let origin = test_origin();
        let mut report = ValidationReport::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors.len(), 1);
        assert!(
            report.errors[0]
                .message
                .contains("max_connections_per_second")
        );
    }

    #[test]
    fn valid_rate_limiting_filter() {
        // Arrange
        let spec = ConnectionRateLimitingFilterSpec {
            window_seconds: 10,
            max_connections_per_second: 100,
        };
        let origin = test_origin();
        let mut report = ValidationReport::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.errors.is_empty());
    }
}
