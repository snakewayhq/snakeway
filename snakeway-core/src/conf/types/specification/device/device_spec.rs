use crate::conf::types::{
    IdentityDeviceSpec, NetworkPolicyDeviceSpec, Origin, RequestFilterDeviceSpec,
    RequestRateLimitingDeviceSpec, StructuredLoggingDeviceSpec, WasmDeviceSpec,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DeviceSpec {
    RequestFilter(RequestFilterDeviceSpec),
    Identity(IdentityDeviceSpec),
    NetworkPolicy(NetworkPolicyDeviceSpec),
    Wasm(WasmDeviceSpec),
    StructuredLogging(StructuredLoggingDeviceSpec),
    RequestRateLimiting(RequestRateLimitingDeviceSpec),
}

impl DeviceSpec {
    pub(crate) fn origin(&self) -> &Origin {
        match self {
            DeviceSpec::RequestFilter(s) => &s.origin,
            DeviceSpec::Identity(s) => &s.origin,
            DeviceSpec::NetworkPolicy(s) => &s.origin,
            DeviceSpec::Wasm(s) => &s.origin,
            DeviceSpec::StructuredLogging(s) => &s.origin,
            DeviceSpec::RequestRateLimiting(s) => &s.origin,
        }
    }
    pub(crate) fn is_enabled(&self) -> bool {
        match self {
            DeviceSpec::RequestFilter(s) => s.enable,
            DeviceSpec::Identity(s) => s.enable,
            DeviceSpec::NetworkPolicy(s) => s.enable,
            DeviceSpec::Wasm(s) => s.enable,
            DeviceSpec::StructuredLogging(s) => s.enable,
            DeviceSpec::RequestRateLimiting(s) => s.enable,
        }
    }
}
