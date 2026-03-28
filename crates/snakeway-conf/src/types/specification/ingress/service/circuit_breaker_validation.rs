use crate::types::{CircuitBreakerSpec, Origin};
use crate::validation::validator::{
    CB_FAILURE_THRESHOLD, CB_HALF_OPEN_MAX_REQUESTS, CB_OPEN_DURATION_MS, CB_SUCCESS_THRESHOLD,
};
use crate::validation::{ValidateSpec, ValidationReport};

impl ValidateSpec for CircuitBreakerSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        CB_FAILURE_THRESHOLD.validate(self.failure_threshold, report, origin);
        CB_OPEN_DURATION_MS.validate(self.open_duration_milliseconds, report, origin);
        CB_HALF_OPEN_MAX_REQUESTS.validate(self.half_open_max_requests, report, origin);
        CB_SUCCESS_THRESHOLD.validate(self.success_threshold, report, origin);
    }
}
