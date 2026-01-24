use crate::conf::types::{
    IdentityDeviceConfig, RequestFilterDeviceConfig, StructuredLoggingDeviceConfig,
    WasmDeviceConfig,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceConfig {
    Wasm(WasmDeviceConfig),
    Identity(IdentityDeviceConfig),
    RequestFilter(RequestFilterDeviceConfig),
    StructuredLogging(StructuredLoggingDeviceConfig),
}

impl DeviceConfig {
    pub fn is_enabled(&self) -> bool {
        match self {
            DeviceConfig::Identity(i) => i.enable,
            DeviceConfig::RequestFilter(r) => r.enable,
            DeviceConfig::StructuredLogging(s) => s.enable,
            DeviceConfig::Wasm(w) => w.enable,
        }
    }
}
