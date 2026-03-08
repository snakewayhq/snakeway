use crate::conf::types::{
    BindAdminSpec, BindSpec, IdentityDeviceSpec, NetworkPolicyDeviceSpec, RequestFilterDeviceSpec,
    RequestRateLimitingDeviceSpec, ServerSpec, ServiceSpec, StaticFilesSpec,
    StructuredLoggingDeviceSpec, WasmDeviceSpec,
};
use serde::{Deserialize, Serialize};

/// Represents the top-level configuration file.
#[derive(Debug, Deserialize, Serialize, Default)]
pub(crate) struct EntrypointSpec {
    pub(crate) server: ServerSpec,
    pub(crate) include: IncludeSpec,
}

/// Represents the include section of the top-level config file.
/// The members are directory paths where sub-configuration files are located.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct IncludeSpec {
    pub(crate) devices: String,
    pub(crate) ingresses: String,
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
#[serde(rename_all = "snake_case")]
pub(crate) struct DevicesFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) request_filter_device: Option<RequestFilterDeviceSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) identity_device: Option<IdentityDeviceSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) network_policy_device: Option<NetworkPolicyDeviceSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) request_rate_limiting_device: Option<RequestRateLimitingDeviceSpec>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) wasm_devices: Vec<WasmDeviceSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) structured_logging_device: Option<StructuredLoggingDeviceSpec>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) struct IngressFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) bind: Option<BindSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) bind_admin: Option<BindAdminSpec>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) services: Vec<ServiceSpec>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) static_files: Vec<StaticFilesSpec>,
}
