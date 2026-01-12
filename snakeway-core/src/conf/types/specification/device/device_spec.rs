use crate::conf::types::{IdentityDeviceSpec, Origin, StructuredLoggingDeviceSpec, WasmDeviceSpec};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceSpec {
    Wasm(WasmDeviceSpec),
    Identity(IdentityDeviceSpec),
    StructuredLogging(StructuredLoggingDeviceSpec),
}

impl DeviceSpec {
    pub fn origin(&self) -> &Origin {
        match self {
            DeviceSpec::Identity(i) => &i.origin,
            DeviceSpec::StructuredLogging(s) => &s.origin,
            DeviceSpec::Wasm(w) => &w.origin,
        }
    }
}
