use crate::types::CircuitBreakerSpec;
use o2o::o2o;
use serde::{Deserialize, Serialize};

#[derive(o2o, Debug, Clone, Deserialize, Serialize, Default)]
#[from_owned(CircuitBreakerSpec)]
pub struct CircuitBreakerConfig {
    pub enable_auto_recovery: bool,
    #[map(~ as u32)]
    pub failure_threshold: u32,
    #[map(~ as u64)]
    pub open_duration_milliseconds: u64,
    #[map(~ as u32)]
    pub half_open_max_requests: u32,
    #[map(~ as u32)]
    pub success_threshold: u32,
    pub count_http_5xx_as_failure: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_spec_maps_all_fields() {
        // Arrange
        let spec = CircuitBreakerSpec {
            enable_auto_recovery: true,
            failure_threshold: 10,
            open_duration_milliseconds: 5000,
            half_open_max_requests: 3,
            success_threshold: 4,
            count_http_5xx_as_failure: false,
        };

        // Act
        let config: CircuitBreakerConfig = spec.into();

        // Assert
        assert!(config.enable_auto_recovery);
        assert_eq!(config.failure_threshold, 10);
        assert_eq!(config.open_duration_milliseconds, 5000);
        assert_eq!(config.half_open_max_requests, 3);
        assert_eq!(config.success_threshold, 4);
        assert!(!config.count_http_5xx_as_failure);
    }
}
