use crate::types::Origin;
use crate::validation::ValidationReport;

#[derive(Debug, Clone)]
pub(crate) struct RangeConstraint<T> {
    pub(crate) min: T,
    pub(crate) max: T,
    pub(crate) label: &'static str,
    pub(crate) units: Option<&'static str>,
}

impl<T> RangeConstraint<T>
where
    T: PartialOrd + std::fmt::Display + Copy,
{
    /// Validate that the given value is within the range of this constraint.
    /// Invalid values are reported as errors in the validation report.
    pub(crate) fn validate(&self, value: T, report: &mut ValidationReport, origin: &Origin) {
        if value < self.min || value > self.max {
            let units = self.units.unwrap_or("");
            report.error(
                format!(
                    "invalid {}: {}{} (must be between {}{} and {}{})",
                    self.label, value, units, self.min, units, self.max, units
                ),
                origin,
                None,
            );
        }
    }
}

pub(crate) const CB_FAILURE_THRESHOLD: RangeConstraint<u32> = RangeConstraint {
    min: 1,
    max: 10_000,
    label: "circuit_breaker.failure_threshold",
    units: None,
};

pub(crate) const CB_OPEN_DURATION_MS: RangeConstraint<u64> = RangeConstraint {
    min: 1,
    max: 60 * 60 * 1000,
    label: "circuit_breaker.open_duration_milliseconds",
    units: Some("ms"),
};

pub(crate) const CB_HALF_OPEN_MAX_REQUESTS: RangeConstraint<u32> = RangeConstraint {
    min: 1,
    max: 10_000,
    label: "circuit_breaker.half_open_max_requests",
    units: None,
};

pub(crate) const CB_SUCCESS_THRESHOLD: RangeConstraint<u32> = RangeConstraint {
    min: 1,
    max: 10_000,
    label: "circuit_breaker.success_threshold",
    units: None,
};

pub(crate) const SERVER_THREADS: RangeConstraint<usize> = RangeConstraint {
    min: 1,
    max: 1024,
    label: "server.threads",
    units: None,
};

pub(crate) const SERVER_TLS_RENEW_WITHIN_DAYS: RangeConstraint<u64> = RangeConstraint {
    min: 7,
    max: 30,
    label: "server.tls.renew_within_days",
    units: Some("days"),
};

pub(crate) const REDIRECT_RESPONSE_CODE: RangeConstraint<u16> = RangeConstraint {
    min: 300,
    max: 399,
    label: "redirect_response_code",
    units: None,
};

pub(crate) const REQUEST_FILTER_DENY_STATUS: RangeConstraint<u16> = RangeConstraint {
    min: 400,
    max: 599,
    label: "request_filter_device.deny_status",
    units: None,
};

pub(crate) const CONNECTION_RATE_LIMITING_REACTION_INTERVAL_IN_SECONDS: RangeConstraint<u16> =
    RangeConstraint {
        min: 1,
        max: 60,
        label: "window_seconds",
        units: Some("seconds"),
    };

pub(crate) const CONNECTION_RATE_LIMITING_FILTER_MAX_CONNECTIONS_PER_SECOND: RangeConstraint<u16> =
    RangeConstraint {
        min: 1,
        max: 30_000,
        label: "max_connections_per_second",
        units: None,
    };

pub(crate) const REQUEST_RATE_LIMITING_DEVICE_WINDOW_SECONDS: RangeConstraint<u16> =
    RangeConstraint {
        min: 1,
        max: 60,
        label: "window_seconds",
        units: Some("seconds"),
    };

pub(crate) const REQUEST_RATE_LIMITING_DEVICE_MAX_REQUESTS_PER_SECOND: RangeConstraint<u16> =
    RangeConstraint {
        min: 1,
        max: 30_000,
        label: "max_requests_per_second",
        units: None,
    };

pub(crate) const IDENTITY_DEVICE_MAX_X_FORWARDED_FOR_LENGTH: RangeConstraint<usize> =
    RangeConstraint {
        min: 1,
        max: 2024,
        label: "max_x_forwarded_for_length",
        units: None,
    };

pub(crate) const IDENTITY_DEVICE_MAX_USER_AGENT_LENGTH: RangeConstraint<usize> = RangeConstraint {
    min: 1,
    max: 4096,
    label: "max_user_agent_length",
    units: None,
};
