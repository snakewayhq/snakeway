use crate::types::{ConnectionRateLimitingFilterSpec, Origin};
use crate::validation::validator::{
    CONNECTION_RATE_LIMITING_FILTER_MAX_CONNECTIONS_PER_SECOND,
    CONNECTION_RATE_LIMITING_REACTION_INTERVAL_IN_SECONDS,
};
use crate::validation::{ValidateSpec, ValidationReport};

impl ValidateSpec for ConnectionRateLimitingFilterSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        CONNECTION_RATE_LIMITING_REACTION_INTERVAL_IN_SECONDS.validate(
            self.window_seconds,
            report,
            origin,
        );

        CONNECTION_RATE_LIMITING_FILTER_MAX_CONNECTIONS_PER_SECOND.validate(
            self.max_connections_per_second,
            report,
            origin,
        );
    }
}
