use crate::types::{Origin, RequestRateLimitingDeviceSpec};
use crate::validation::{
    RangeConstraint, ValidateSpec, ValidationReport, range_constraint, validate_range_field,
};

range_constraint!(MAX_REQUESTS_PER_SECOND, u16, min: 1, max: 30_000);
range_constraint!(WINDOW_SECONDS, u16, min: 1, max: 60, units: "seconds");

impl ValidateSpec for RequestRateLimitingDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        validate_range_field!(
            MAX_REQUESTS_PER_SECOND,
            self.max_requests_per_second,
            report,
            origin
        );
        validate_range_field!(WINDOW_SECONDS, self.window_seconds, report, origin);
    }
}
