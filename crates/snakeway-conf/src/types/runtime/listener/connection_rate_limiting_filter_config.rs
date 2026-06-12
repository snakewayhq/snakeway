use crate::types::ConnectionRateLimitingFilterSpec;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Deserialize, Default, Serialize, Clone, PartialEq)]
pub struct ConnectionRateLimitingFilterConfig {
    pub max_connections_per_second: f64,
    pub reaction_interval: Duration,
}

impl From<&ConnectionRateLimitingFilterSpec> for ConnectionRateLimitingFilterConfig {
    fn from(spec: &ConnectionRateLimitingFilterSpec) -> Self {
        Self {
            max_connections_per_second: spec.max_connections_per_second.value as f64,
            reaction_interval: Duration::from_secs(spec.window_seconds.value as u64),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_spec_converts_duration() {
        // Arrange
        use confval::provenance::Located;
        let spec = ConnectionRateLimitingFilterSpec {
            max_connections_per_second: Located::detached(50),
            window_seconds: Located::detached(10),
        };

        // Act
        let config: ConnectionRateLimitingFilterConfig = (&spec).into();

        // Assert
        assert_eq!(config.max_connections_per_second, 50.0);
        assert_eq!(config.reaction_interval, Duration::from_secs(10));
    }
}
