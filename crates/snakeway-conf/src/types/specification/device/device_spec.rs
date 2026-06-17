use crate::types::{
    IdentityDeviceSpec, NetworkPolicyDeviceSpec, RequestFilterDeviceSpec,
    RequestRateLimitingDeviceSpec, StructuredLoggingDeviceSpec, WasmDeviceSpec,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceSpec {
    RequestFilter(RequestFilterDeviceSpec),
    Identity(IdentityDeviceSpec),
    NetworkPolicy(NetworkPolicyDeviceSpec),
    Wasm(WasmDeviceSpec),
    StructuredLogging(StructuredLoggingDeviceSpec),
    RequestRateLimiting(RequestRateLimitingDeviceSpec),
}

impl DeviceSpec {
    pub fn is_enabled(&self) -> bool {
        match self {
            DeviceSpec::RequestFilter(s) => s.enable.value,
            DeviceSpec::Identity(s) => s.enable.value,
            DeviceSpec::NetworkPolicy(s) => s.enable.value,
            DeviceSpec::Wasm(s) => s.enable.value,
            DeviceSpec::StructuredLogging(s) => s.enable.value,
            DeviceSpec::RequestRateLimiting(s) => s.enable.value,
        }
    }
}
