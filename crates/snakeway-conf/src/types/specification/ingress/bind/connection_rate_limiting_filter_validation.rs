use crate::types::{ConnectionRateLimitingFilterSpec, Origin};
use crate::validation::validator::{
    CONNECTION_RATE_LIMITING_FILTER_MAX_CONNECTIONS_PER_SECOND,
    CONNECTION_RATE_LIMITING_REACTION_INTERVAL_IN_SECONDS, validate_range,
};
use crate::validation::{ValidateSpec, ValidationReport};

impl ValidateSpec for ConnectionRateLimitingFilterSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        validate_range(
            self.window_seconds,
            &CONNECTION_RATE_LIMITING_REACTION_INTERVAL_IN_SECONDS,
            report,
            origin,
        );

        validate_range(
            self.max_connections_per_second,
            &CONNECTION_RATE_LIMITING_FILTER_MAX_CONNECTIONS_PER_SECOND,
            report,
            origin,
        );
    }
}
