use crate::types::{Origin, RequestRateLimitingDeviceSpec};
use crate::validation::validator::{
    REQUEST_RATE_LIMITING_DEVICE_MAX_REQUESTS_PER_SECOND,
    REQUEST_RATE_LIMITING_DEVICE_WINDOW_SECONDS, validate_range,
};
use crate::validation::{ValidateSpec, ValidationReport};

impl ValidateSpec for RequestRateLimitingDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        validate_range(
            self.max_requests_per_second,
            &REQUEST_RATE_LIMITING_DEVICE_MAX_REQUESTS_PER_SECOND,
            report,
            origin,
        );
        validate_range(
            self.window_seconds,
            &REQUEST_RATE_LIMITING_DEVICE_WINDOW_SECONDS,
            report,
            origin,
        );
    }
}
