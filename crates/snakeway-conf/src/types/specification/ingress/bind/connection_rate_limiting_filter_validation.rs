use crate::types::{ConnectionRateLimitingFilterSpec, Origin};
use crate::validation::{RangeConstraint, ValidateSpec, ValidationReport, range_constraint};

range_constraint!(REACTION_INTERVAL_IN_SECONDS, u16, min: 1, max: 60, label: "window_seconds", units: "seconds");
range_constraint!(FILTER_MAX_CONNECTIONS_PER_SECOND, u16, min: 1, max: 30_000, label: "max_connections_per_second");

impl ValidateSpec for ConnectionRateLimitingFilterSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        REACTION_INTERVAL_IN_SECONDS.validate(self.window_seconds, report, origin);

        FILTER_MAX_CONNECTIONS_PER_SECOND.validate(self.max_connections_per_second, report, origin);
    }
}
