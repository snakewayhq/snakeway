use crate::types::HealthCheckSpec;

use super::HealthCheckConfig;

impl From<HealthCheckSpec> for HealthCheckConfig {
    fn from(spec: HealthCheckSpec) -> Self {
        Self {
            enable: spec.enable,
            failure_threshold: spec.failure_threshold,
            unhealthy_cooldown_seconds: spec.unhealthy_cooldown_seconds,
        }
    }
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
