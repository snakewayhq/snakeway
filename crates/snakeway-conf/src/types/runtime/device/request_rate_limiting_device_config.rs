use crate::types::RequestRateLimitingDeviceSpec;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::time::Duration;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RequestRateLimitingDeviceConfig {
    pub enable: bool,
    pub reaction_interval: Duration,
    pub max_requests_per_second: f64,
    pub paths: SmallVec<[String; 4]>,
}

impl From<RequestRateLimitingDeviceSpec> for RequestRateLimitingDeviceConfig {
    fn from(spec: RequestRateLimitingDeviceSpec) -> Self {
        Self {
            enable: spec.enable.value,
            reaction_interval: Duration::from_secs(spec.window_seconds.value as u64),
            max_requests_per_second: spec.max_requests_per_second.value as f64,
            paths: spec.paths.into_iter().map(|p| p.value).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_spec_converts_duration_and_rate() {
        // Arrange
        use confval::provenance::Located;
        let spec = RequestRateLimitingDeviceSpec {
            enable: Located::detached(true),
            max_requests_per_second: Located::detached(100),
            window_seconds: Located::detached(60),
            paths: vec![],
        };

        // Act
        let config: RequestRateLimitingDeviceConfig = spec.into();

        // Assert
        assert!(config.enable);
        assert_eq!(config.reaction_interval, Duration::from_secs(60));
        assert_eq!(config.max_requests_per_second, 100.0);
    }
}
