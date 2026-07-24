use crate::types::RequestRateLimitingDeviceSpec;
use confval::prelude::{Lower, Report, Validate, ValidateNested, narrow};
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

impl Lower<RequestRateLimitingDeviceSpec> for RequestRateLimitingDeviceConfig
where
    RequestRateLimitingDeviceSpec: Validate + ValidateNested,
{
    fn lower(spec: &RequestRateLimitingDeviceSpec, report: &mut Report) -> Option<Self> {
        Some(Self {
            enable: spec.enable.value,
            reaction_interval: narrow::i64_secs_to_duration(&spec.window_seconds, report)?,
            max_requests_per_second: narrow::i64_to_f64(&spec.max_requests_per_second, report)?,
            paths: spec.paths.iter().map(|p| p.value.clone()).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_spec_converts_duration_and_rate() {
        // Arrange
        use confval::prelude::Located;
        let spec = RequestRateLimitingDeviceSpec {
            enable: Located::detached(true),
            max_requests_per_second: Located::detached(100),
            window_seconds: Located::detached(60),
            paths: vec![],
        };

        // Act
        let config = RequestRateLimitingDeviceConfig::lower(&spec, &mut Report::new()).unwrap();

        // Assert
        assert!(config.enable);
        assert_eq!(config.reaction_interval, Duration::from_secs(60));
        assert_eq!(config.max_requests_per_second, 100.0);
    }
}
