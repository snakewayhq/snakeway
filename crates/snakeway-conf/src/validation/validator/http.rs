use crate::types::OriginDeprecated;
use crate::validation::ValidationReportDeprecated;
use http::{HeaderName, Method};

pub(crate) fn validate_http_header_name(
    header: &str,
    report: &mut ValidationReportDeprecated,
    origin: &OriginDeprecated,
) {
    if HeaderName::from_bytes(header.as_bytes()).is_err() {
        report.invalid_http_header_name(header, origin);
    }
}

pub(crate) fn validate_http_method(
    method: &str,
    report: &mut ValidationReportDeprecated,
    origin: &OriginDeprecated,
) {
    if Method::from_bytes(method.as_bytes()).is_err() {
        report.invalid_http_method(method, origin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_header_name() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let origin = OriginDeprecated::test("test");

        // Act
        validate_http_header_name("content-type", &mut report, &origin);

        // Assert
        assert!(!report.has_violations());
    }

    #[test]
    fn invalid_header_name() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let origin = OriginDeprecated::test("test");

        // Act
        validate_http_header_name("invalid header!", &mut report, &origin);

        // Assert
        assert!(report.has_violations());
        assert_eq!(report.errors.len(), 1);
        assert!(
            report.errors[0]
                .message
                .contains("invalid HTTP header name"),
            "expected error to contain 'invalid HTTP header name', got: {}",
            report.errors[0].message
        );
    }

    #[test]
    fn valid_http_method() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let origin = OriginDeprecated::test("test");

        // Act
        validate_http_method("GET", &mut report, &origin);

        // Assert
        assert!(!report.has_violations());
    }

    #[test]
    fn invalid_http_method() {
        // Arrange
        let mut report = ValidationReportDeprecated::default();
        let origin = OriginDeprecated::test("test");

        // Act
        validate_http_method("INVALID METHOD", &mut report, &origin);

        // Assert
        assert!(report.has_violations());
        assert_eq!(report.errors.len(), 1);
        assert!(
            report.errors[0].message.contains("invalid HTTP method"),
            "expected error to contain 'invalid HTTP method', got: {}",
            report.errors[0].message
        );
    }
}
