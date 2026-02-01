use crate::conf::types::{
    IdentityDeviceSpec, NetworkPolicyDeviceSpec, Origin, RequestFilterDeviceSpec,
    StructuredLoggingDeviceSpec, WasmDeviceSpec,
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
}

impl DeviceSpec {
    pub fn origin(&self) -> &Origin {
        match self {
            DeviceSpec::RequestFilter(s) => &s.origin,
            DeviceSpec::Identity(s) => &s.origin,
            DeviceSpec::NetworkPolicy(s) => &s.origin,
            DeviceSpec::Wasm(s) => &s.origin,
            DeviceSpec::StructuredLogging(s) => &s.origin,
        }
    }
}
