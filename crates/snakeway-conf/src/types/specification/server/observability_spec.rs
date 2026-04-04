use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct ObservabilitySpec {
    pub otel: Option<OtelSpec>,
}

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct OtelSpec {
    pub enable: bool,
    pub endpoint: String,
    pub service_name: String,
    #[serde(default = "default_sampling_ratio")]
    pub sampling_ratio: f64,
}

fn default_sampling_ratio() -> f64 {
    1.0
}
