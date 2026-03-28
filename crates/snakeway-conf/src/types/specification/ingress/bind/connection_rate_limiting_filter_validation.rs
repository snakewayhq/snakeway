use crate::types::{ConnectionRateLimitingFilterSpec, Origin};
use crate::validation::{RangeConstraint, ValidateSpec, ValidationReport};

const CONNECTION_RATE_LIMITING_REACTION_INTERVAL_IN_SECONDS: RangeConstraint<u16> =
    RangeConstraint {
        min: 1,
        max: 60,
        label: "window_seconds",
        units: Some("seconds"),
    };

const CONNECTION_RATE_LIMITING_FILTER_MAX_CONNECTIONS_PER_SECOND: RangeConstraint<u16> =
    RangeConstraint {
        min: 1,
        max: 30_000,
        label: "max_connections_per_second",
        units: None,
    };

impl ValidateSpec for ConnectionRateLimitingFilterSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        CONNECTION_RATE_LIMITING_REACTION_INTERVAL_IN_SECONDS.validate(
            self.window_seconds,
            report,
            origin,
        );

        CONNECTION_RATE_LIMITING_FILTER_MAX_CONNECTIONS_PER_SECOND.validate(
            self.max_connections_per_second,
            report,
            origin,
        );
    }
}
