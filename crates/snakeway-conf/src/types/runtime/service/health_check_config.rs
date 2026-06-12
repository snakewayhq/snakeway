use crate::types::HealthCheckSpec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HealthCheckConfig {
    pub enable: bool,
    pub failure_threshold: u64,
    pub unhealthy_cooldown_seconds: u64,
}

impl From<&HealthCheckSpec> for HealthCheckConfig {
    fn from(spec: &HealthCheckSpec) -> Self {
        Self {
            enable: spec.enable.value,
            failure_threshold: spec.failure_threshold.value as u64,
            unhealthy_cooldown_seconds: spec.unhealthy_cooldown_seconds.value as u64,
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
        let spec = HealthCheckSpec {
            enable: Located::detached(true),
            failure_threshold: Located::detached(7),
            unhealthy_cooldown_seconds: Located::detached(30),
        };

        // Act
        let config: HealthCheckConfig = (&spec).into();

        // Assert
        assert!(config.enable);
        assert_eq!(config.failure_threshold, 7);
        assert_eq!(config.unhealthy_cooldown_seconds, 30);
    }
}
