use crate::types::{Origin, RequestRateLimitingDeviceSpec};
use crate::validation::validator::{
    REQUEST_RATE_LIMITING_DEVICE_MAX_REQUESTS_PER_SECOND,
    REQUEST_RATE_LIMITING_DEVICE_WINDOW_SECONDS,
};
use crate::validation::{ValidateSpec, ValidationReport};

impl ValidateSpec for RequestRateLimitingDeviceSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        REQUEST_RATE_LIMITING_DEVICE_MAX_REQUESTS_PER_SECOND.validate(
            self.max_requests_per_second,
            report,
            origin,
        );
        REQUEST_RATE_LIMITING_DEVICE_WINDOW_SECONDS.validate(self.window_seconds, report, origin);
    }
}
