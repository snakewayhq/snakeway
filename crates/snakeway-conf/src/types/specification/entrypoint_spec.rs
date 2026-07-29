use crate::types::{
    IdentityDeviceSpec, NetworkPolicyDeviceSpec, RequestFilterDeviceSpec,
    RequestRateLimitingDeviceSpec, ServerSpec, StructuredLoggingDeviceSpec, WasmDeviceSpec,
};
use confval::format::{
    Field, Fields, FromFields, ToFields, parse_single_struct, report_missing_field,
    report_unknown_field,
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

impl FromFields for EntrypointSpec {
    fn from_fields(fields: &Fields, report: &mut Report) -> Option<Self> {
        let mut server: Option<Located<ServerSpec>> = None;
        let mut server_seen: Option<Span> = None;
        let mut include: Option<Located<IncludeSpec>> = None;
        let mut include_seen = None;

        for field in fields.iter() {
            match field.name.as_str() {
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

/// The write-path counterpart of the handwritten `FromFields`: the two
/// top-level blocks, in the order an entrypoint file lists them.
impl ToFields for EntrypointSpec {
    fn to_fields(&self) -> Fields {
        Fields::detached(vec![
            Field::detached_block("server", self.server.to_fields()),
            Field::detached_block("include", self.include.to_fields()),
        ])
    }

    fn to_template(&self) -> Fields {
        Fields::detached(vec![
            Field::detached_block("server", self.server.to_template()),
            Field::detached_block("include", self.include.to_template()),
        ])
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_fields_round_trips_default_entrypoint() {
        // Arrange
        let spec = EntrypointSpec::default();
        let mut report = Report::new();

        // Act
        let round_tripped = EntrypointSpec::from_fields(&spec.to_fields(), &mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let round_tripped = round_tripped.unwrap();
        assert_eq!(round_tripped.include.devices.value, "device.d/*.hcl");
        assert_eq!(round_tripped.include.ingresses.value, "ingress.d/*.hcl");
        assert_eq!(
            round_tripped.server.version.value,
            spec.server.version.value
        );
    }
}
