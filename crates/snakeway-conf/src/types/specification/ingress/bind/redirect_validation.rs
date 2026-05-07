use crate::types::{OriginDeprecated, RedirectSpec};
use crate::validation::validator::is_valid_port;
use crate::validation::{
    RangeConstraint, ValidateSpec, ValidationReportDeprecated, range_constraint,
    validate_range_field,
};

range_constraint!(RESPONSE_CODE, u16, min: 300, max: 399);

impl ValidateSpec for RedirectSpec {
    fn validate(&self, origin: &OriginDeprecated, report: &mut ValidationReportDeprecated) {
        if !is_valid_port(self.port) {
            report.invalid_port(self.port, origin);
        }

        validate_range_field!(RESPONSE_CODE, self.status, report, origin);
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{OriginDeprecated, RedirectSpec};
    use crate::validation::{ValidateSpec, ValidationReportDeprecated};

    fn test_origin() -> OriginDeprecated {
        OriginDeprecated::test("redirect_http_to_https")
    }

    #[test]
    fn valid_3xx_status_produces_no_errors() {
        // Arrange
        let spec = RedirectSpec {
            port: 8080,
            status: 308,
        };
        let origin = test_origin();
        let mut report = ValidationReportDeprecated::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.errors.is_empty());
    }

    #[test]
    fn valid_non_3xx_status_produces_error_bottom_of_range() {
        // Arrange
        let status = 299;
        let expected_error = format!("invalid status: {status} (must be between 300 and 399)");
        let spec = RedirectSpec { port: 8080, status };
        let origin = test_origin();
        let mut report = ValidationReportDeprecated::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors[0].message, expected_error);
    }

    #[test]
    fn valid_non_3xx_status_produces_error_top_of_range() {
        // Arrange
        let status = 400;
        let expected_error = format!("invalid status: {status} (must be between 300 and 399)");
        let spec = RedirectSpec { port: 8080, status };
        let origin = test_origin();
        let mut report = ValidationReportDeprecated::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors[0].message, expected_error);
    }

    #[test]
    fn invalid_port_produces_error() {
        // Arrange
        let spec = RedirectSpec {
            port: 0,
            status: 308,
        };
        let origin = test_origin();
        let mut report = ValidationReportDeprecated::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors[0].message, "invalid port: 0");
    }
}
