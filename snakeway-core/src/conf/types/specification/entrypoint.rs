use crate::conf::types::{
    BindAdminSpec, BindSpec, IdentityDeviceSpec, NetworkPolicyDeviceSpec, RequestFilterDeviceSpec,
    RequestRateLimitingDeviceSpec, ServerSpec, ServiceSpec, StaticFilesSpec,
    StructuredLoggingDeviceSpec, WasmDeviceSpec,
};
use serde::{Deserialize, Serialize};

/// Represents the top-level configuration file.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct EntrypointSpec {
    pub server: ServerSpec,
    pub include: IncludeSpec,
}

/// Represents the include section of the top-level config file.
/// The members are directory paths where sub-configuration files are located.
#[derive(Debug, Deserialize, Serialize)]
pub struct IncludeSpec {
    pub devices: String,
    pub ingresses: String,
}

impl Default for IncludeSpec {
    fn default() -> Self {
        Self {
            devices: "device.d/*.hcl".to_string(),
            ingresses: "ingress.d/*.hcl".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub(crate) struct DevicesFile {
    pub(crate) request_filter_device: Option<RequestFilterDeviceSpec>,
    pub(crate) identity_device: Option<IdentityDeviceSpec>,
    pub(crate) network_policy_device: Option<NetworkPolicyDeviceSpec>,
    pub(crate) request_rate_limiting_device: Option<RequestRateLimitingDeviceSpec>,
    #[serde(default)]
    pub(crate) wasm_devices: Vec<WasmDeviceSpec>,
    pub(crate) structured_logging_device: Option<StructuredLoggingDeviceSpec>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub(crate) struct IngressFile {
    pub(crate) bind: Option<BindSpec>,

    pub(crate) bind_admin: Option<BindAdminSpec>,

    #[serde(default)]
    pub(crate) services: Vec<ServiceSpec>,

    #[serde(default)]
    pub(crate) static_files: Vec<StaticFilesSpec>,
}
