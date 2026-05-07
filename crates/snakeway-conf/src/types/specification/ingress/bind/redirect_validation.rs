use crate::types::{HclOrigin, RedirectSpec};
use crate::validation::ValidationReportExt;
use crate::validation::validator::is_valid_port;
use confval::{
    RangeConstraint, ValidateSpec, ValidationReport, range_constraint, validate_range_field,
};

range_constraint!(RESPONSE_CODE, u16, min: 300, max: 399);

impl ValidateSpec<HclOrigin> for RedirectSpec {
    fn validate(&self, origin: &HclOrigin, report: &mut ValidationReport<HclOrigin>) {
        if !is_valid_port(self.port) {
            report.invalid_port(self.port, origin);
        }

        validate_range_field!(RESPONSE_CODE, self.status, report, origin);
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{HclOrigin, RedirectSpec};
    use confval::{ValidateSpec, ValidationReport};

    fn test_origin() -> HclOrigin {
        HclOrigin::test("redirect_http_to_https")
    }

    #[test]
    fn valid_3xx_status_produces_no_errors() {
        // Arrange
        let spec = RedirectSpec {
            port: 8080,
            status: 308,
        };
        let origin = test_origin();
        let mut report = ValidationReport::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(report.errors().is_empty());
    }

    #[test]
    fn valid_non_3xx_status_produces_error_bottom_of_range() {
        // Arrange
        let status = 299;
        let spec = RedirectSpec { port: 8080, status };
        let origin = test_origin();
        let mut report = ValidationReport::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(
            report.errors()[0]
                .message
                .contains("status must be at least 300")
        );
    }

    #[test]
    fn valid_non_3xx_status_produces_error_top_of_range() {
        // Arrange
        let status = 400;
        let spec = RedirectSpec { port: 8080, status };
        let origin = test_origin();
        let mut report = ValidationReport::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert!(
            report.errors()[0]
                .message
                .contains("status must be at most 399")
        );
    }

    #[test]
    fn invalid_port_produces_error() {
        // Arrange
        let spec = RedirectSpec {
            port: 0,
            status: 308,
        };
        let origin = test_origin();
        let mut report = ValidationReport::default();

        // Act
        spec.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors()[0].message, "invalid port: 0");
    }
}
