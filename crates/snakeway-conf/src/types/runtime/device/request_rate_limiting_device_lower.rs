use crate::types::RequestRateLimitingDeviceSpec;
use std::time::Duration;

use super::RequestRateLimitingDeviceConfig;

impl From<RequestRateLimitingDeviceSpec> for RequestRateLimitingDeviceConfig {
    fn from(spec: RequestRateLimitingDeviceSpec) -> Self {
        Self {
            enable: spec.enable,
            reaction_interval: Duration::from_secs(spec.window_seconds as u64),
            max_requests_per_second: spec.max_requests_per_second as f64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Origin;

    #[test]
    fn from_spec_converts_duration_and_rate() {
        // Arrange
        let spec = RequestRateLimitingDeviceSpec {
            origin: Origin::default(),
            enable: true,
            max_requests_per_second: 100,
            window_seconds: 60,
        };

        // Act
        let config: RequestRateLimitingDeviceConfig = spec.into();

        // Assert
        assert!(config.enable);
        assert_eq!(config.reaction_interval, Duration::from_secs(60));
        assert_eq!(config.max_requests_per_second, 100.0);
    }
}
