use crate::types::ConnectionRateLimitingFilterSpec;
use confval::provenance::{Located, Report, narrow};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Deserialize, Default, Serialize, Clone, PartialEq, confval::Config)]
#[confval(lower_from = ConnectionRateLimitingFilterSpec)]
pub struct ConnectionRateLimitingFilterConfig {
    #[confval(lower(from = max_connections_per_second, with = i64_to_f64))]
    pub max_connections_per_second: f64,
    #[confval(lower(from = window_seconds, with = secs_to_duration))]
    pub reaction_interval: Duration,
}

fn i64_to_f64(value: &Located<i64>, _report: &mut Report) -> Option<f64> {
    Some(value.value as f64)
}

fn secs_to_duration(value: &Located<i64>, report: &mut Report) -> Option<Duration> {
    narrow::i64_to_u64(value, report).map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::provenance::Lower;

    #[test]
    fn lower_converts_duration() {
        // Arrange
        let spec = ConnectionRateLimitingFilterSpec {
            max_connections_per_second: Located::detached(50),
            window_seconds: Located::detached(10),
        };

        // Act
        let config = ConnectionRateLimitingFilterConfig::lower(&spec, &mut Report::new()).unwrap();

        // Assert
        assert_eq!(config.max_connections_per_second, 50.0);
        assert_eq!(config.reaction_interval, Duration::from_secs(10));
    }
}
