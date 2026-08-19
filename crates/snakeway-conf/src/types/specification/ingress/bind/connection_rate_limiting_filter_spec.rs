use confval::prelude::{Located, Report, Validate};
use confval::{RangeConstraint, range_constraint};
use serde::Serialize;

range_constraint!(REACTION_INTERVAL_IN_SECONDS, i64, min: 1, max: 60, units: "seconds");
range_constraint!(MAX_CONNECTIONS_PER_SECOND, i64, min: 1, max: 30_000);

#[derive(Debug, Serialize, Default, Clone, confval::Spec)]
pub struct ConnectionRateLimitingFilterSpec {
    pub max_connections_per_second: Located<i64>,
    pub window_seconds: Located<i64>,
}

impl Validate for ConnectionRateLimitingFilterSpec {
    fn validate(&self, report: &mut Report) {
        REACTION_INTERVAL_IN_SECONDS.check_located(&self.window_seconds, "window_seconds", report);
        MAX_CONNECTIONS_PER_SECOND.check_located(
            &self.max_connections_per_second,
            "max_connections_per_second",
            report,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(
        window_seconds: i64,
        max_connections_per_second: i64,
    ) -> ConnectionRateLimitingFilterSpec {
        ConnectionRateLimitingFilterSpec {
            max_connections_per_second: Located::detached(max_connections_per_second),
            window_seconds: Located::detached(window_seconds),
        }
    }

    #[test]
    fn window_seconds_below_range() {
        // Arrange
        let spec = spec(0, 100);
        let mut report = Report::new();

        // Act
        spec.validate(&mut report);

        // Assert
        assert_eq!(report.issues().len(), 1);
        assert!(report.issues()[0].message.contains("window_seconds"));
    }

    #[test]
    fn window_seconds_above_range() {
        // Arrange
        let spec = spec(61, 100);
        let mut report = Report::new();

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report.issues()[0]
                .message
                .contains("window_seconds must be at most 60")
        );
    }

    #[test]
    fn max_connections_out_of_range() {
        // Arrange
        let spec = spec(10, 0);
        let mut report = Report::new();

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(
            report.issues()[0]
                .message
                .contains("max_connections_per_second must be at least 1")
        );
    }

    #[test]
    fn valid_filter_produces_no_errors() {
        // Arrange
        let spec = spec(10, 100);
        let mut report = Report::new();

        // Act
        spec.validate(&mut report);

        // Assert
        assert!(!report.has_issues());
    }
}
