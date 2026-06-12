use crate::types::HclInt;
use confval::provenance::{Located, Report};
use confval::{RangeConstraint, range_constraint};
use serde::Serialize;

range_constraint!(FAILURE_THRESHOLD, i64, min: 1, max: 10_000);
range_constraint!(OPEN_DURATION_MS, i64, min: 1, max: 60 * 60 * 1000, units: "ms");
range_constraint!(HALF_OPEN_MAX_REQUESTS, i64, min: 1, max: 10_000);
range_constraint!(SUCCESS_THRESHOLD, i64, min: 1, max: 10_000);

#[derive(Debug, Clone, Serialize, confval::Spec)]
pub struct CircuitBreakerSpec {
    /// Enable circuit breaking auto recovery for this service.
    #[confval(default)]
    pub enable_auto_recovery: Located<bool>,

    /// Failures in the "closed" state before opening the circuit.
    #[confval(default = 5)]
    pub failure_threshold: Located<HclInt>,

    /// How long to keep the circuit open before allowing probes.
    #[confval(default = 10_000)]
    pub open_duration_milliseconds: Located<HclInt>,

    /// How many simultaneous probe requests are allowed in half-open.
    /// (Start with 1; keep it simple and safe.)
    #[confval(default = 1)]
    pub half_open_max_requests: Located<HclInt>,

    /// How many successful probes close the circuit again.
    #[confval(default = 2)]
    pub success_threshold: Located<HclInt>,

    /// Whether HTTP 5xx responses count as failures for the circuit.
    #[confval(default = true)]
    pub count_http_5xx_as_failure: Located<bool>,
}

impl Default for CircuitBreakerSpec {
    fn default() -> Self {
        Self {
            enable_auto_recovery: Located::detached(false),
            failure_threshold: Located::detached(5),
            open_duration_milliseconds: Located::detached(10_000),
            half_open_max_requests: Located::detached(1),
            success_threshold: Located::detached(2),
            count_http_5xx_as_failure: Located::detached(true),
        }
    }
}

pub(crate) fn validate_circuit_breaker(spec: &CircuitBreakerSpec, report: &mut Report) {
    FAILURE_THRESHOLD.check_located(&spec.failure_threshold, "failure_threshold", report);
    OPEN_DURATION_MS.check_located(
        &spec.open_duration_milliseconds,
        "open_duration_milliseconds",
        report,
    );
    HALF_OPEN_MAX_REQUESTS.check_located(
        &spec.half_open_max_requests,
        "half_open_max_requests",
        report,
    );
    SUCCESS_THRESHOLD.check_located(&spec.success_threshold, "success_threshold", report);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_circuit_breaker() {
        // Arrange
        let mut report = Report::new();
        let spec = CircuitBreakerSpec {
            enable_auto_recovery: Located::detached(true),
            ..Default::default()
        };

        // Act
        validate_circuit_breaker(&spec, &mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn failure_threshold_out_of_range() {
        // Arrange
        let mut report = Report::new();
        let spec = CircuitBreakerSpec {
            enable_auto_recovery: Located::detached(true),
            failure_threshold: Located::detached(0),
            ..Default::default()
        };

        // Act
        validate_circuit_breaker(&spec, &mut report);

        // Assert
        let error = report.issues().first().expect("expected an error");
        assert!(
            error
                .message
                .contains("failure_threshold must be at least 1")
        );
    }

    #[test]
    fn open_duration_out_of_range() {
        // Arrange
        let mut report = Report::new();
        let spec = CircuitBreakerSpec {
            enable_auto_recovery: Located::detached(true),
            open_duration_milliseconds: Located::detached(0),
            ..Default::default()
        };

        // Act
        validate_circuit_breaker(&spec, &mut report);

        // Assert
        let error = report.issues().first().expect("expected an error");
        assert!(
            error
                .message
                .contains("open_duration_milliseconds must be at least 1")
        );
    }
}
