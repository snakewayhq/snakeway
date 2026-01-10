use crate::conf::types::{
    IdentityDeviceConfig, Origin, StructuredLoggingDeviceConfig, WasmDeviceConfig,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceConfig {
    Wasm(WasmDeviceConfig),
    Identity(IdentityDeviceConfig),
    StructuredLogging(StructuredLoggingDeviceConfig),
}

impl DeviceConfig {
    pub fn is_enabled(&self) -> bool {
        match self {
            DeviceConfig::Identity(i) => i.enable,
            DeviceConfig::StructuredLogging(s) => s.enable,
            DeviceConfig::Wasm(w) => w.enable,
        }
    }

    pub fn origin(&self) -> &Origin {
        match self {
            DeviceConfig::Identity(i) => &i.origin,
            DeviceConfig::StructuredLogging(s) => &s.origin,
            DeviceConfig::Wasm(w) => &w.origin,
        }
    }
}
