use crate::types::{Origin, RequestRateLimitingDeviceSpec};
use crate::validation::{RangeConstraint, ValidateSpec, ValidationReport, range_constraint};

range_constraint!(WINDOW_SECONDS, u16, min: 1, max: 60, label: "window_seconds", units: "seconds");
range_constraint!(MAX_REQUESTS_PER_SECOND, u16, min: 1, max: 30_000, label: "max_requests_per_second");

impl ValidateSpec for RequestRateLimitingDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        MAX_REQUESTS_PER_SECOND.validate(self.max_requests_per_second, report, origin);
        WINDOW_SECONDS.validate(self.window_seconds, report, origin);
    }
}
