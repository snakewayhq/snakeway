use crate::types::{
    IdentityDeviceSpec, NetworkPolicyDeviceSpec, RequestFilterDeviceSpec,
    RequestRateLimitingDeviceSpec, StructuredLoggingDeviceSpec, WasmDeviceSpec,
};
use confval::format::{Field, Fields, ToFields};
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

    /// The block name a device file spells this device with.
    pub fn block_name(&self) -> &'static str {
        match self {
            DeviceSpec::RequestFilter(_) => "request_filter_device",
            DeviceSpec::Identity(_) => "identity_device",
            DeviceSpec::NetworkPolicy(_) => "network_policy_device",
            DeviceSpec::Wasm(_) => "wasm_devices",
            DeviceSpec::StructuredLogging(_) => "structured_logging_device",
            DeviceSpec::RequestRateLimiting(_) => "request_rate_limiting_device",
        }
    }

    fn inner(&self) -> &dyn ToFields {
        match self {
            DeviceSpec::RequestFilter(s) => s,
            DeviceSpec::Identity(s) => s,
            DeviceSpec::NetworkPolicy(s) => s,
            DeviceSpec::Wasm(s) => s,
            DeviceSpec::StructuredLogging(s) => s,
            DeviceSpec::RequestRateLimiting(s) => s,
        }
    }
}

/// A device emits as the one named block its device file spells it with.
impl ToFields for DeviceSpec {
    fn to_fields(&self) -> Fields {
        Fields::detached(vec![Field::detached_block(
            self.block_name(),
            self.inner().to_fields(),
        )])
    }

    fn to_source_fields(&self) -> Fields {
        Fields::detached(vec![Field::detached_block(
            self.block_name(),
            self.inner().to_source_fields(),
        )])
    }
}
