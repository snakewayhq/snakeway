use crate::types::{
    IdentityDeviceSpec, NetworkPolicyDeviceSpec, RequestFilterDeviceSpec,
    RequestRateLimitingDeviceSpec, ServerSpec, StructuredLoggingDeviceSpec, WasmDeviceSpec,
};
use confval::prelude::{Located, Report, Validate};
use serde::Serialize;

/// Represents the top-level configuration file.
#[derive(Debug, Serialize, confval::Spec)]
pub struct EntrypointSpec {
    #[confval(nested)]
    pub server: Located<ServerSpec>,
    #[confval(nested)]
    pub include: Located<IncludeSpec>,
}

impl Default for EntrypointSpec {
    fn default() -> Self {
        Self {
            server: Located::detached(ServerSpec::default()),
            include: Located::detached(IncludeSpec::default()),
        }
    }
}

impl Validate for EntrypointSpec {
    fn validate(&self, _report: &mut Report) {}
}

/// The include section of the top-level config file.
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

impl Validate for IncludeSpec {
    fn validate(&self, _report: &mut Report) {}
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
    use confval::format::hcl::{emit_hcl, parse_hcl};
    use confval::format::{FromFields, ToFields};
    use confval::prelude::SourceMap;

    fn parse_devices(input: &str) -> (Report, Option<DevicesFile>) {
        let mut sources = SourceMap::new();
        let mut report = Report::new();
        let id = sources.add("devices.hcl", input);
        let spec = parse_hcl::<DevicesFile>(&sources, id, &mut report);
        (report, spec)
    }

    #[test]
    fn wasm_device_native_label_fills_name() {
        // Arrange
        let input = r#"wasm_devices "auth" {
  enable = true
  path = "./a.wasm"
  fail_policy = "open"
}
"#;

        // Act
        let (report, spec) = parse_devices(input);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let devices = spec.unwrap().wasm_devices;
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].value.name.value, "auth");
    }

    #[test]
    fn wasm_device_name_attribute_still_fills_name() {
        // Arrange
        let input = r#"wasm_devices {
  name = "auth"
  enable = true
  path = "./a.wasm"
  fail_policy = "open"
}
"#;

        // Act
        let (report, spec) = parse_devices(input);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        assert_eq!(spec.unwrap().wasm_devices[0].value.name.value, "auth");
    }

    #[test]
    fn wasm_device_label_and_attribute_together_report() {
        // Arrange
        let input = r#"wasm_devices "auth" {
  name = "other"
  enable = true
  path = "./a.wasm"
  fail_policy = "open"
}
"#;

        // Act
        let (report, spec) = parse_devices(input);

        // Assert
        let _ = spec;
        assert!(
            report.has_issues(),
            "a label and a name attribute together must be reported"
        );
    }

    #[test]
    fn wasm_device_label_round_trips_through_spec_walk_as_attribute() {
        // Arrange
        let input = r#"wasm_devices "auth" {
  enable = true
  path = "./a.wasm"
  fail_policy = "open"
}
"#;
        let (report, spec) = parse_devices(input);
        assert!(!report.has_issues(), "issues: {:?}", report.issues());

        // Act
        let emitted = emit_hcl(&spec.unwrap().to_fields()).unwrap();

        // Assert
        assert!(emitted.contains("name = \"auth\""), "emitted: {emitted}");
        let (round_report, round_tripped) = parse_devices(&emitted);
        assert!(
            !round_report.has_issues(),
            "issues: {:?}",
            round_report.issues()
        );
        assert_eq!(
            round_tripped.unwrap().wasm_devices[0].value.name.value,
            "auth"
        );
    }

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
