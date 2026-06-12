use crate::types::CircuitBreakerSpec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CircuitBreakerConfig {
    pub enable_auto_recovery: bool,
    pub failure_threshold: u32,
    pub open_duration_milliseconds: u64,
    pub half_open_max_requests: u32,
    pub success_threshold: u32,
    pub count_http_5xx_as_failure: bool,
}

impl From<&CircuitBreakerSpec> for CircuitBreakerConfig {
    fn from(spec: &CircuitBreakerSpec) -> Self {
        Self {
            enable_auto_recovery: spec.enable_auto_recovery.value,
            failure_threshold: spec.failure_threshold.value as u32,
            open_duration_milliseconds: spec.open_duration_milliseconds.value as u64,
            half_open_max_requests: spec.half_open_max_requests.value as u32,
            success_threshold: spec.success_threshold.value as u32,
            count_http_5xx_as_failure: spec.count_http_5xx_as_failure.value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_spec_maps_all_fields() {
        // Arrange
        use confval::provenance::Located;
        let spec = CircuitBreakerSpec {
            enable_auto_recovery: Located::detached(true),
            failure_threshold: Located::detached(10),
            open_duration_milliseconds: Located::detached(5000),
            half_open_max_requests: Located::detached(3),
            success_threshold: Located::detached(4),
            count_http_5xx_as_failure: Located::detached(false),
        };

        // Act
        let config: CircuitBreakerConfig = (&spec).into();

        // Assert
        assert!(config.enable_auto_recovery);
        assert_eq!(config.failure_threshold, 10);
        assert_eq!(config.open_duration_milliseconds, 5000);
        assert_eq!(config.half_open_max_requests, 3);
        assert_eq!(config.success_threshold, 4);
        assert!(!config.count_http_5xx_as_failure);
    }
}
