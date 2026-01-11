use crate::conf::parse::parse_devices;
use crate::conf::types::DeviceConfig;
use std::fs;
use tempfile::tempdir;

#[test]
fn parse_identity_device_file() {
    // Arrange
    let dir = tempdir().unwrap();
    let path = dir.path().join("identity.hcl");

    fs::write(
        &path,
        r#"
identity_device = {
  enable = true
  trusted_proxies = ["127.0.0.1/32"]
  enable_geoip = false
  enable_user_agent = false
  ua_engine = "woothee"
}
"#,
    )
    .unwrap();

    // Act
    let devices = parse_devices(&path).unwrap();

    // Assert
    assert_eq!(devices.len(), 1);
    assert!(matches!(devices[0], DeviceConfig::Identity(_)));
}

#[test]
fn parse_structured_logging_device_file() {
    // Arrange
    let dir = tempdir().unwrap();
    let path = dir.path().join("structured_logging.hcl");

    fs::write(
        &path,
        r#"
structured_logging_device = {
  enable = true
  include_headers = false
  allowed_headers = []
  redacted_headers = []
  level = "info"
  include_identity = false
  identity_fields = []
}
"#,
    )
    .unwrap();

    // Act
    let devices = parse_devices(&path).unwrap();

    // Assert
    assert_eq!(devices.len(), 1);
    assert!(matches!(devices[0], DeviceConfig::StructuredLogging(_)));
}
