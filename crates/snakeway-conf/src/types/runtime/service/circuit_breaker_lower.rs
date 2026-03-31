use crate::types::CircuitBreakerSpec;

use super::CircuitBreakerConfig;

impl From<CircuitBreakerSpec> for CircuitBreakerConfig {
    fn from(spec: CircuitBreakerSpec) -> Self {
        Self {
            enable_auto_recovery: spec.enable_auto_recovery,
            failure_threshold: spec.failure_threshold,
            open_duration_milliseconds: spec.open_duration_milliseconds,
            half_open_max_requests: spec.half_open_max_requests,
            success_threshold: spec.success_threshold,
            count_http_5xx_as_failure: spec.count_http_5xx_as_failure,
        }
    }
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
