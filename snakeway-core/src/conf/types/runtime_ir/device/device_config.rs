use crate::conf::types::{IdentityDeviceConfig, StructuredLoggingDeviceConfig, WasmDeviceConfig};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
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
}

//     /// Device-specific configuration blob
//     pub config: toml::Value,
