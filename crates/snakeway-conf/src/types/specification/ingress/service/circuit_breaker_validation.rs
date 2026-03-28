use crate::types::{CircuitBreakerSpec, Origin};
use crate::validation::{RangeConstraint, ValidateSpec, ValidationReport};

const FAILURE_THRESHOLD: RangeConstraint<u32> = RangeConstraint {
    min: 1,
    max: 10_000,
    label: "circuit_breaker.failure_threshold",
    units: None,
};

const OPEN_DURATION_MS: RangeConstraint<u64> = RangeConstraint {
    min: 1,
    max: 60 * 60 * 1000,
    label: "circuit_breaker.open_duration_milliseconds",
    units: Some("ms"),
};

const HALF_OPEN_MAX_REQUESTS: RangeConstraint<u32> = RangeConstraint {
    min: 1,
    max: 10_000,
    label: "circuit_breaker.half_open_max_requests",
    units: None,
};

const SUCCESS_THRESHOLD: RangeConstraint<u32> = RangeConstraint {
    min: 1,
    max: 10_000,
    label: "circuit_breaker.success_threshold",
    units: None,
};

impl ValidateSpec for CircuitBreakerSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        FAILURE_THRESHOLD.validate(self.failure_threshold, report, origin);
        OPEN_DURATION_MS.validate(self.open_duration_milliseconds, report, origin);
        HALF_OPEN_MAX_REQUESTS.validate(self.half_open_max_requests, report, origin);
        SUCCESS_THRESHOLD.validate(self.success_threshold, report, origin);
    }
}
