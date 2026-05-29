use crate::types::RequestRateLimitingDeviceSpec;
use o2o::o2o;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::time::Duration;

#[derive(o2o, Debug, Clone, Default, Deserialize, Serialize)]
#[from_owned(RequestRateLimitingDeviceSpec)]
pub struct RequestRateLimitingDeviceConfig {
    pub enable: bool,
    #[from(window_seconds, Duration::from_secs(~ as u64))]
    pub reaction_interval: Duration,
    #[from(max_requests_per_second, ~ as f64)]
    pub max_requests_per_second: f64,
    #[map(~.into_iter().collect())]
    pub paths: SmallVec<[String; 4]>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::HclOrigin;

    #[test]
    fn from_spec_converts_duration_and_rate() {
        // Arrange
        let spec = RequestRateLimitingDeviceSpec {
            origin: HclOrigin::default(),
            enable: true,
            max_requests_per_second: 100,
            window_seconds: 60,
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
