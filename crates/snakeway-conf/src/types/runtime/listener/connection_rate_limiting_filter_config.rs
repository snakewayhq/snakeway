use crate::types::ConnectionRateLimitingFilterSpec;
use o2o::o2o;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(o2o, Debug, Deserialize, Default, Serialize, Clone, PartialEq)]
#[from_owned(ConnectionRateLimitingFilterSpec)]
pub struct ConnectionRateLimitingFilterConfig {
    #[from(max_connections_per_second, ~ as f64)]
    pub max_connections_per_second: f64,
    #[from(window_seconds, Duration::from_secs(~ as u64))]
    pub reaction_interval: Duration,
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
