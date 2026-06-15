use crate::types::{
    IdentityDeviceSpec, NetworkPolicyDeviceSpec, RequestFilterDeviceSpec,
    RequestRateLimitingDeviceSpec, ServerSpec, StructuredLoggingDeviceSpec, WasmDeviceSpec,
};
use confval::format::hcl::{
    Fields, FromHcl, parse_single_struct, report_missing_field, report_unknown_field,
};
use confval::prelude::{Located, Report, Span};
use serde::Serialize;

/// Represents the top-level configuration file.
#[derive(Debug, Serialize, Default)]
pub struct EntrypointSpec {
    pub server: ServerSpec,
    pub include: IncludeSpec,
}

/// Represents the include section of the top-level config file.
/// The members are directory paths where sub-configuration files are located.
#[derive(Debug, Serialize, confval::Spec)]
pub struct IncludeSpec {
    pub devices: Located<String>,
    pub ingresses: Located<String>,
}

impl Default for IncludeSpec {
    fn default() -> Self {
        Self {
            devices: Located::detached("device.d/*.hcl".to_string()),
            ingresses: Located::detached("ingress.d/*.hcl".to_string()),
        }
    }
}

impl FromHcl for EntrypointSpec {
    fn from_hcl(fields: &Fields<'_>, report: &mut Report) -> Option<Self> {
        let mut server: Option<Located<ServerSpec>> = None;
        let mut server_seen: Option<Span> = None;
        let mut include: Option<Located<IncludeSpec>> = None;
        let mut include_seen = None;

        for field in fields.iter() {
            match field.name {
                "server" => {
                    parse_single_struct(&mut server, &mut server_seen, "server", field, report)
                }
                "include" => {
                    parse_single_struct(&mut include, &mut include_seen, "include", field, report)
                }
                _ => report_unknown_field(field, report),
            }
        }

        if server_seen.is_none() {
            report_missing_field("server", fields.enclosing(), report);
        }
        if include_seen.is_none() {
            report_missing_field("include", fields.enclosing(), report);
        }

        Some(EntrypointSpec {
            server: server?.value,
            include: include?.value,
        })
    }
}

#[derive(Debug, Serialize, Default, confval::Spec)]
#[serde(rename_all = "snake_case")]
pub struct DevicesFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub request_filter_device: Option<Located<RequestFilterDeviceSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub identity_device: Option<Located<IdentityDeviceSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub network_policy_device: Option<Located<NetworkPolicyDeviceSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub request_rate_limiting_device: Option<Located<RequestRateLimitingDeviceSpec>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[confval(nested)]
    pub wasm_devices: Vec<Located<WasmDeviceSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub structured_logging_device: Option<Located<StructuredLoggingDeviceSpec>>,
}
