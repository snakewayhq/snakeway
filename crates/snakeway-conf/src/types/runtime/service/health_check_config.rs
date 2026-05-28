use crate::types::HealthCheckSpec;
use o2o::o2o;
use serde::{Deserialize, Serialize};

#[derive(o2o, Debug, Clone, Deserialize, Serialize, Default)]
#[from_owned(HealthCheckSpec)]
pub struct HealthCheckConfig {
    pub enable: bool,
    pub failure_threshold: u64,
    pub unhealthy_cooldown_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_spec_maps_all_fields() {
        // Arrange
        let spec = HealthCheckSpec {
            enable: true,
            failure_threshold: 7,
            unhealthy_cooldown_seconds: 30,
        };

        // Act
        let config: HealthCheckConfig = spec.into();

        // Assert
        assert!(config.enable);
        assert_eq!(config.failure_threshold, 7);
        assert_eq!(config.unhealthy_cooldown_seconds, 30);
    }
}
