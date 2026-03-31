use crate::types::ConnectionRateLimitingFilterSpec;
use std::time::Duration;

use super::ConnectionRateLimitingFilterConfig;

impl From<ConnectionRateLimitingFilterSpec> for ConnectionRateLimitingFilterConfig {
    fn from(spec: ConnectionRateLimitingFilterSpec) -> Self {
        Self {
            max_connections_per_second: spec.max_connections_per_second as f64,
            reaction_interval: Duration::from_secs(spec.window_seconds as u64),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_spec_converts_duration() {
        // Arrange
        let spec = ConnectionRateLimitingFilterSpec {
            max_connections_per_second: 50,
            window_seconds: 10,
        };

        // Act
        let config: ConnectionRateLimitingFilterConfig = spec.into();

        // Assert
        assert_eq!(config.max_connections_per_second, 50.0);
        assert_eq!(config.reaction_interval, Duration::from_secs(10));
    }
}
