use crate::conf::types::{
    IdentityDeviceConfig, NetworkPolicyDeviceConfig, OtelDeviceConfig, RequestFilterDeviceConfig,
    RequestRateLimitingDeviceConfig, StructuredLoggingDeviceConfig, WasmDeviceConfig,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceConfig {
    RequestFilter(RequestFilterDeviceConfig),
    Identity(IdentityDeviceConfig),
    NetworkPolicy(NetworkPolicyDeviceConfig),
    Otel(OtelDeviceConfig),
    Wasm(WasmDeviceConfig),
    StructuredLogging(StructuredLoggingDeviceConfig),
    RequestRateLimiting(RequestRateLimitingDeviceConfig),
}

impl DeviceConfig {
    pub fn is_enabled(&self) -> bool {
        match self {
            DeviceConfig::RequestFilter(r) => r.enable,
            DeviceConfig::Identity(i) => i.enable,
            DeviceConfig::NetworkPolicy(i) => i.enable,
            DeviceConfig::Otel(o) => o.enable,
            DeviceConfig::Wasm(w) => w.enable,
            DeviceConfig::StructuredLogging(s) => s.enable,
            DeviceConfig::RequestRateLimiting(r) => r.enable,
        }
    }
}
