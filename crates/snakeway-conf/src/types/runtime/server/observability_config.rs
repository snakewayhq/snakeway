use crate::types::{ObservabilitySpec, OtelSpec};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Default, Serialize, confval::Config)]
#[confval(lower_from = ObservabilitySpec)]
pub struct ObservabilityConfig {
    #[confval(nested)]
    pub otel: Option<OtelConfig>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default, Serialize, confval::Config)]
#[confval(lower_from = OtelSpec)]
pub struct OtelConfig {
    pub enable: bool,
    pub endpoint: String,
    pub service_name: String,
    pub sampling_ratio: f64,
}
