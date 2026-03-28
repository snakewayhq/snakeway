use crate::types::{CircuitBreakerSpec, Origin};
use crate::validation::{
    RangeConstraint, ValidateSpec, ValidationReport, range_constraint, validate_range_field,
};

range_constraint!(FAILURE_THRESHOLD, u32, min: 1, max: 10_000);
range_constraint!(OPEN_DURATION_MS, u64, min: 1, max: 60 * 60 * 1000, units: "ms");
range_constraint!(HALF_OPEN_MAX_REQUESTS, u32, min: 1, max: 10_000);
range_constraint!(SUCCESS_THRESHOLD, u32, min: 1, max: 10_000);

impl ValidateSpec for CircuitBreakerSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
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
