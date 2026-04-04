use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct ObservabilitySpec {
    pub logging: Option<LoggingSpec>,
    pub otel: Option<OtelSpec>,
}

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct LoggingSpec {
    pub enable: bool,
    pub log_dir: Option<String>,
}

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct OtelSpec {
    pub enable: bool,
    pub endpoint: String,
    pub service_name: String,
    pub sampling: SamplingTypeSpec,
}

#[derive(Debug, Deserialize, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SamplingTypeSpec {
    #[default]
    ParentBased,
}
