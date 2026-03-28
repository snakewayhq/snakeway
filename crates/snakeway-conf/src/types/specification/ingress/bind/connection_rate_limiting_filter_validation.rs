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
