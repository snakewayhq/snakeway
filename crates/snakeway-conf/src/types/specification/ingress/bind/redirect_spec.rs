use crate::validation::validator::is_valid_port;
use confval::prelude::{Located, Report, Validate};
use confval::{RangeConstraint, range_constraint};
use serde::Serialize;

range_constraint!(RESPONSE_CODE, i64, min: 300, max: 399);

#[derive(Debug, Serialize, Clone, confval::Spec)]
pub struct RedirectSpec {
    pub port: Located<i64>,
    pub status: Located<i64>,
}

pub(crate) fn report_invalid_port(port: &Located<i64>, report: &mut Report) {
    report
        .error(format!("invalid port: {}", port.value))
        .at(port.span)
        .help("ports must be in the range 1–65535")
        .emit();
}

impl Validate for RedirectSpec {
    fn validate(&self, report: &mut Report) {
        if !is_valid_port(self.port.value) {
            report_invalid_port(&self.port, report);
        }

        RESPONSE_CODE.check_located(&self.status, "status", report);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn redirect(port: i64, status: i64) -> RedirectSpec {
        RedirectSpec {
            port: Located::detached(port),
            status: Located::detached(status),
        }
    }

    #[test]
    fn valid_3xx_status_produces_no_errors() {
        // Arrange
        let spec = redirect(8080, 308);
        let mut report = Report::new();

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(!report.has_errors());
    }

    #[test]
    fn valid_non_3xx_status_produces_error_bottom_of_range() {
        // Arrange
        let spec = redirect(8080, 299);
        let mut report = Report::new();

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report.issues()[0]
                .message
                .contains("status must be at least 300")
        );
    }

    #[test]
    fn valid_non_3xx_status_produces_error_top_of_range() {
        // Arrange
        let spec = redirect(8080, 400);
        let mut report = Report::new();

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report.issues()[0]
                .message
                .contains("status must be at most 399")
        );
    }

    #[test]
    fn invalid_port_produces_error() {
        // Arrange
        let spec = redirect(0, 308);
        let mut report = Report::new();

        // Act
        spec.validate(&mut report);

        // Assert
        assert_eq!(report.issues()[0].message, "invalid port: 0");
    }
}
