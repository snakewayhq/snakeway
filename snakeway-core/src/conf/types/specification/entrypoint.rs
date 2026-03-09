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
#[serde(rename_all = "snake_case")]
pub struct DevicesFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_filter_device: Option<RequestFilterDeviceSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity_device: Option<IdentityDeviceSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_policy_device: Option<NetworkPolicyDeviceSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_rate_limiting_device: Option<RequestRateLimitingDeviceSpec>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub wasm_devices: Vec<WasmDeviceSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_logging_device: Option<StructuredLoggingDeviceSpec>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct IngressFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind: Option<BindSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind_admin: Option<BindAdminSpec>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub services: Vec<ServiceSpec>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub static_files: Vec<StaticFilesSpec>,
}
