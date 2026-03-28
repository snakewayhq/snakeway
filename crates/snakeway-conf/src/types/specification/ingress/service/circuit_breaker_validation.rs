use crate::types::{CircuitBreakerSpec, Origin};
use crate::validation::{RangeConstraint, ValidateSpec, ValidationReport, range_constraint};

range_constraint!(FAILURE_THRESHOLD, u32, min: 1, max: 10_000, label: "circuit_breaker.failure_threshold");
range_constraint!(OPEN_DURATION_MS, u64, min: 1, max: 60 * 60 * 1000, label: "circuit_breaker.open_duration_milliseconds", units: "ms");
range_constraint!(HALF_OPEN_MAX_REQUESTS, u32, min: 1, max: 10_000, label: "circuit_breaker.half_open_max_requests");
range_constraint!(SUCCESS_THRESHOLD, u32, min: 1, max: 10_000, label: "circuit_breaker.success_threshold");

impl ValidateSpec for CircuitBreakerSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        FAILURE_THRESHOLD.validate(self.failure_threshold, report, origin);
        OPEN_DURATION_MS.validate(self.open_duration_milliseconds, report, origin);
        HALF_OPEN_MAX_REQUESTS.validate(self.half_open_max_requests, report, origin);
        SUCCESS_THRESHOLD.validate(self.success_threshold, report, origin);
    }
}
