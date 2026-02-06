use crate::conf::types::{
    IdentityDeviceConfig, NetworkPolicyDeviceConfig, RequestFilterDeviceConfig,
    StructuredLoggingDeviceConfig, WasmDeviceConfig,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceConfig {
    RequestFilter(RequestFilterDeviceConfig),
    Identity(IdentityDeviceConfig),
    NetworkPolicy(NetworkPolicyDeviceConfig),
    Wasm(WasmDeviceConfig),
    StructuredLogging(StructuredLoggingDeviceConfig),
}

impl DeviceConfig {
    pub fn is_enabled(&self) -> bool {
        match self {
            DeviceConfig::RequestFilter(r) => r.enable,
            DeviceConfig::Identity(i) => i.enable,
            DeviceConfig::NetworkPolicy(i) => i.enable,
            DeviceConfig::Wasm(w) => w.enable,
            DeviceConfig::StructuredLogging(s) => s.enable,
        }
    }
}
