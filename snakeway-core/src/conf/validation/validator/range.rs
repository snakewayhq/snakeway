use crate::conf::types::Origin;
use crate::conf::validation::ValidationReport;

#[derive(Debug, Clone)]
pub struct RangeConstraint<T> {
    pub min: T,
    pub max: T,
    pub label: &'static str,
    pub units: Option<&'static str>,
}

pub const CB_FAILURE_THRESHOLD: RangeConstraint<u32> = RangeConstraint {
    min: 1,
    max: 10_000,
    label: "circuit_breaker.failure_threshold",
    units: None,
};

pub const CB_OPEN_DURATION_MS: RangeConstraint<u64> = RangeConstraint {
    min: 1,
    max: 60 * 60 * 1000,
    label: "circuit_breaker.open_duration_milliseconds",
    units: Some("ms"),
};

pub const CB_HALF_OPEN_MAX_REQUESTS: RangeConstraint<u32> = RangeConstraint {
    min: 1,
    max: 10_000,
    label: "circuit_breaker.half_open_max_requests",
    units: None,
};

pub const CB_SUCCESS_THRESHOLD: RangeConstraint<u32> = RangeConstraint {
    min: 1,
    max: 10_000,
    label: "circuit_breaker.success_threshold",
    units: None,
};

pub const SERVER_THREADS: RangeConstraint<usize> = RangeConstraint {
    min: 1,
    max: 1024,
    label: "server.threads",
    units: None,
};

pub const REDIRECT_RESPONSE_CODE: RangeConstraint<u16> = RangeConstraint {
    min: 300,
    max: 399,
    label: "redirect_response_code",
    units: None,
};

pub const REQUEST_FILTER_DENY_STATUS: RangeConstraint<u16> = RangeConstraint {
    min: 400,
    max: 599,
    label: "request_filter_device.deny_status",
    units: None,
};

pub fn validate_range<T>(
    value: T,
    constraint: &RangeConstraint<T>,
    report: &mut ValidationReport,
    origin: &Origin,
) where
    T: PartialOrd + std::fmt::Display,
{
    if value < constraint.min || value > constraint.max {
        let units = constraint.units.unwrap_or("");
        report.error(
            format!(
                "invalid {}: {}{} (must be between {}{} and {}{})",
                constraint.label, value, units, constraint.min, units, constraint.max, units
            ),
            origin,
            None,
        );
    }
}
