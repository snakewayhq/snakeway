use crate::types::{DeviceSpec, DevicesFile};
use confval::prelude::Located;

/// Flattens a parsed devices file into the device list, each wrapped with
/// its own structure span.
pub(crate) fn flatten_devices(file: DevicesFile) -> Vec<Located<DeviceSpec>> {
    let mut device_config = Vec::new();

    if let Some(identity) = file.identity_device {
        device_config.push(Located::new(
            DeviceSpec::Identity(identity.value),
            identity.span,
        ));
    }

    if let Some(network_policy) = file.network_policy_device {
        device_config.push(Located::new(
            DeviceSpec::NetworkPolicy(network_policy.value),
            network_policy.span,
        ));
    }

    if let Some(request_rate_limiting) = file.request_rate_limiting_device {
        device_config.push(Located::new(
            DeviceSpec::RequestRateLimiting(request_rate_limiting.value),
            request_rate_limiting.span,
        ));
    }

    if let Some(logging) = file.structured_logging_device {
        device_config.push(Located::new(
            DeviceSpec::StructuredLogging(logging.value),
            logging.span,
        ));
    }

    if let Some(request_filter) = file.request_filter_device {
        device_config.push(Located::new(
            DeviceSpec::RequestFilter(request_filter.value),
            request_filter.span,
        ));
    }

    for device in file.wasm_devices {
        device_config.push(Located::new(DeviceSpec::Wasm(device.value), device.span));
    }

    device_config
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::format::hcl::parse_hcl;
    use confval::prelude::{Report, SourceMap};

    fn parse(input: &str) -> (Report, Option<DevicesFile>) {
        let mut sources = SourceMap::new();
        let mut report = Report::new();
        let id = sources.add("devices.hcl", input);
        let file = parse_hcl::<DevicesFile>(&sources, id, &mut report);
        (report, file)
    }

    #[test]
    fn parse_identity_device_file() {
        // Arrange
        let input = r#"
identity_device = {
  enable = true
  trusted_proxies = ["127.0.0.1/32"]
  enable_geoip = false
  enable_user_agent = false
  ua_engine = "woothee"
}
"#;

        // Act
        let (report, file) = parse(input);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let devices = flatten_devices(file.unwrap());
        assert_eq!(devices.len(), 1);
        assert!(matches!(devices[0].value, DeviceSpec::Identity(_)));
    }

    #[test]
    fn parse_structured_logging_device_file() {
        // Arrange
        let input = r#"
structured_logging_device = {
  enable = true
  include_headers = false
  allowed_headers = []
  redacted_headers = []
  level = "info"
  include_identity = false
  identity_fields = []
}
"#;

        // Act
        let (report, file) = parse(input);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let devices = flatten_devices(file.unwrap());
        assert_eq!(devices.len(), 1);
        assert!(matches!(devices[0].value, DeviceSpec::StructuredLogging(_)));
    }

    #[test]
    fn parse_wasm_device_array() {
        // Arrange
        let input = r#"
wasm_devices = [
  { name = "dev-a", enable = false, path = "./a.wasm", fail_policy = "open" },
  { name = "dev-b", enable = true,  path = "./b.wasm", fail_policy = "closed" }
]
"#;

        // Act
        let (report, file) = parse(input);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
        let devices = flatten_devices(file.unwrap());
        assert_eq!(devices.len(), 2);
        assert!(
            devices
                .iter()
                .all(|d| matches!(d.value, DeviceSpec::Wasm(_)))
        );
    }

    #[test]
    fn parse_devices_empty_file_is_ok() {
        // Arrange / Act
        let (report, file) = parse("");

        // Assert
        assert!(!report.has_issues());
        assert!(flatten_devices(file.unwrap()).is_empty());
    }

    #[test]
    fn parse_devices_unknown_field_is_reported() {
        // Arrange / Act
        let (report, _) = parse("identity_devic = {\n  enable = true\n}\n");

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "unknown field: identity_devic")
        );
    }
}
