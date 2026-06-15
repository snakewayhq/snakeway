use crate::types::HealthCheckSpec;
use confval::prelude::narrow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default, confval::Config)]
#[confval(lower_from = HealthCheckSpec)]
pub struct HealthCheckConfig {
    pub enable: bool,
    #[confval(lower(from = failure_threshold, with = narrow::i64_to_u64))]
    pub failure_threshold: u64,
    #[confval(lower(from = unhealthy_cooldown_seconds, with = narrow::i64_to_u64))]
    pub unhealthy_cooldown_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::prelude::{Located, Lower, Report};

    #[test]
    fn lower_maps_all_fields() {
        // Arrange
        let spec = HealthCheckSpec {
            enable: Located::detached(true),
            failure_threshold: Located::detached(7),
            unhealthy_cooldown_seconds: Located::detached(30),
        };

        // Act
        let config = HealthCheckConfig::lower(&spec, &mut Report::new()).unwrap();

        // Assert
        assert!(config.enable);
        assert_eq!(config.failure_threshold, 7);
        assert_eq!(config.unhealthy_cooldown_seconds, 30);
    }
}
