use crate::types::{Origin, RequestRateLimitingDeviceSpec};
use crate::validation::{RangeConstraint, ValidateSpec, ValidationReport};

const WINDOW_SECONDS: RangeConstraint<u16> = RangeConstraint {
    min: 1,
    max: 60,
    label: "window_seconds",
    units: Some("seconds"),
};

const MAX_REQUESTS_PER_SECOND: RangeConstraint<u16> = RangeConstraint {
    min: 1,
    max: 30_000,
    label: "max_requests_per_second",
    units: None,
};

impl ValidateSpec for RequestRateLimitingDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        MAX_REQUESTS_PER_SECOND.validate(self.max_requests_per_second, report, origin);
        WINDOW_SECONDS.validate(self.window_seconds, report, origin);
    }
}
