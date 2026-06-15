use crate::types::CircuitBreakerSpec;
use confval::provenance::narrow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default, confval::Config)]
#[confval(lower_from = CircuitBreakerSpec)]
pub struct CircuitBreakerConfig {
    pub enable_auto_recovery: bool,
    #[confval(lower(from = failure_threshold, with = narrow::i64_to_u32))]
    pub failure_threshold: u32,
    #[confval(lower(from = open_duration_milliseconds, with = narrow::i64_to_u64))]
    pub open_duration_milliseconds: u64,
    #[confval(lower(from = half_open_max_requests, with = narrow::i64_to_u32))]
    pub half_open_max_requests: u32,
    #[confval(lower(from = success_threshold, with = narrow::i64_to_u32))]
    pub success_threshold: u32,
    pub count_http_5xx_as_failure: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::provenance::{Located, Lower, Report};

    #[test]
    fn lower_maps_all_fields() {
        // Arrange
        let spec = CircuitBreakerSpec {
            enable_auto_recovery: Located::detached(true),
            failure_threshold: Located::detached(10),
            open_duration_milliseconds: Located::detached(5000),
            half_open_max_requests: Located::detached(3),
            success_threshold: Located::detached(4),
            count_http_5xx_as_failure: Located::detached(false),
        };

        // Act
        let config = CircuitBreakerConfig::lower(&spec, &mut Report::new()).unwrap();

        // Assert
        assert!(config.enable_auto_recovery);
        assert_eq!(config.failure_threshold, 10);
        assert_eq!(config.open_duration_milliseconds, 5000);
        assert_eq!(config.half_open_max_requests, 3);
        assert_eq!(config.success_threshold, 4);
        assert!(!config.count_http_5xx_as_failure);
    }
}
