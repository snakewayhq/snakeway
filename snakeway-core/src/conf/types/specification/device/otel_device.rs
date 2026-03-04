use crate::conf::types::Origin;
use crate::enrichment::identity_field::IdentityField;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct OtelDeviceSpec {
    #[serde(skip)]
    pub origin: Origin,

    pub enable: bool,

    /// OTLP gRPC endpoint. Falls back to OTEL_EXPORTER_OTLP_ENDPOINT env var.
    #[serde(default)]
    pub endpoint: Option<String>,

    /// service.name resource attribute. Defaults to "snakeway".
    #[serde(default = "default_service_name")]
    pub service_name: String,

    /// Identity fields to attach as span/metric attributes.
    #[serde(default)]
    pub identity_fields: Vec<IdentityField>,
}

fn default_service_name() -> String {
    "snakeway".to_string()
}
