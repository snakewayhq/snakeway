use crate::conf::types::OtelDeviceSpec;
use crate::enrichment::identity_field::IdentityField;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct OtelDeviceConfig {
    pub enable: bool,
    pub endpoint: Option<String>,
    pub service_name: String,
    pub identity_fields: Vec<IdentityField>,
}

impl From<OtelDeviceSpec> for OtelDeviceConfig {
    fn from(spec: OtelDeviceSpec) -> Self {
        Self {
            enable: spec.enable,
            endpoint: spec.endpoint,
            service_name: spec.service_name,
            identity_fields: spec.identity_fields,
        }
    }
}
