use crate::types::{CircuitBreakerSpec, Origin};
use crate::validation::validator::{
    CB_FAILURE_THRESHOLD, CB_HALF_OPEN_MAX_REQUESTS, CB_OPEN_DURATION_MS, CB_SUCCESS_THRESHOLD,
    validate_range,
};
use crate::validation::{ValidateSpec, ValidationReport};

impl ValidateSpec for CircuitBreakerSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        validate_range(
            self.failure_threshold,
            &CB_FAILURE_THRESHOLD,
            report,
            origin,
        );
        validate_range(
            self.open_duration_milliseconds,
            &CB_OPEN_DURATION_MS,
            report,
            origin,
        );
        validate_range(
            self.half_open_max_requests,
            &CB_HALF_OPEN_MAX_REQUESTS,
            report,
            origin,
        );
        validate_range(
            self.success_threshold,
            &CB_SUCCESS_THRESHOLD,
            report,
            origin,
        );
    }
}
