use crate::conf::types::{DeviceSpec, IdentityDeviceSpec, WasmDeviceSpec};
use crate::conf::validation::{ValidationReport, validate_devices};
use std::path::PathBuf;

#[test]
fn validate_wasm_device_valid() {
    // Arrange
    let mut report = ValidationReport::default();
    let dir = tempfile::tempdir().unwrap();

    let wasm_file = dir.path().join("plugin.wasm");
    std::fs::write(&wasm_file, "dummy wasm").unwrap();

    let device = DeviceSpec::Wasm(WasmDeviceSpec {
        enable: true,
        path: wasm_file,
        ..Default::default()
    });

    // Act
    validate_devices(&[device], &mut report);

    // Assert
    assert!(!report.has_violations());
}

#[test]
fn validate_wasm_device_disabled_skips_validation() {
    // Arrange
    let mut report = ValidationReport::default();
    let device = DeviceSpec::Wasm(WasmDeviceSpec {
        enable: false,
        path: PathBuf::from("/non/existent/path"),
        ..Default::default()
    });
    let devices = vec![device];

    // Act
    validate_devices(&devices, &mut report);

    // Assert
    assert!(!report.has_violations());
}

#[test]
fn validate_wasm_device_path_empty() {
    // Arrange
    let mut report = ValidationReport::default();
    let device = DeviceSpec::Wasm(WasmDeviceSpec {
        enable: true,
        path: PathBuf::from(""),
        ..Default::default()
    });
    let devices = vec![device];

    // Act
    validate_devices(&devices, &mut report);

    // Assert
    assert!(report.has_violations());
    let error_messages: Vec<String> = report.errors.iter().map(|e| e.message.clone()).collect();
    assert!(
        error_messages
            .iter()
            .any(|m| m.contains("wasm device path is empty"))
    );
    assert!(
        error_messages
            .iter()
            .any(|m| m.contains("wasm device path does not exist"))
    );
}

#[test]
fn validate_wasm_device_path_does_not_exist() {
    // Arrange
    let mut report = ValidationReport::default();
    let device = DeviceSpec::Wasm(WasmDeviceSpec {
        enable: true,
        path: PathBuf::from("/non/existent/path/to/wasm"),
        ..Default::default()
    });
    let devices = vec![device];

    // Act
    validate_devices(&devices, &mut report);

    // Assert
    assert!(report.has_violations());
    let error_messages: Vec<String> = report.errors.iter().map(|e| e.message.clone()).collect();
    assert!(
        error_messages
            .iter()
            .any(|m| m.contains("wasm device path does not exist"))
    );
}

#[test]
fn validate_wasm_device_path_is_not_a_file() {
    let mut report = ValidationReport::default();
    let dir = tempfile::tempdir().unwrap();

    let device = DeviceSpec::Wasm(WasmDeviceSpec {
        enable: true,
        path: dir.path().to_path_buf(), // directory, not file
        ..Default::default()
    });

    validate_devices(&[device], &mut report);

    assert!(
        report
            .errors
            .iter()
            .any(|e| e.message.contains("wasm device path is not a file"))
    );
}

#[test]
fn validate_identity_device_valid() {
    let mut report = ValidationReport::default();
    let dir = tempfile::tempdir().unwrap();

    let geoip = dir.path().join("geoip.mmdb");
    std::fs::write(&geoip, "dummy").unwrap();

    let device = DeviceSpec::Identity(IdentityDeviceSpec {
        enable: true,
        trusted_proxies: vec!["127.0.0.1/32".to_string(), "10.0.0.0/8".to_string()],
        enable_geoip: true,
        geoip_city_db: Some(geoip),
        ..Default::default()
    });

    validate_devices(&[device], &mut report);

    assert!(!report.has_violations());
}

#[test]
fn validate_identity_device_invalid_trusted_proxy() {
    // Arrange
    let mut report = ValidationReport::default();
    let device = DeviceSpec::Identity(IdentityDeviceSpec {
        enable: true,
        trusted_proxies: vec!["not-an-ip".to_string()],
        ..Default::default()
    });
    let devices = vec![device];

    // Act
    validate_devices(&devices, &mut report);

    // Assert
    assert!(report.has_violations());
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.message.contains("invalid trusted proxy: not-an-ip"))
    );
}

#[test]
fn validate_identity_device_trusted_proxy_catch_all_v4() {
    // Arrange
    let mut report = ValidationReport::default();
    let device = DeviceSpec::Identity(IdentityDeviceSpec {
        enable: true,
        trusted_proxies: vec!["0.0.0.0/0".to_string()],
        ..Default::default()
    });
    let devices = vec![device];

    // Act
    validate_devices(&devices, &mut report);

    // Assert
    assert!(report.has_violations());
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.message.contains("must not contain a catch-all network"))
    );
}

#[test]
fn validate_identity_device_trusted_proxy_catch_all_v6() {
    // Arrange
    let mut report = ValidationReport::default();
    let device = DeviceSpec::Identity(IdentityDeviceSpec {
        enable: true,
        trusted_proxies: vec!["::/0".to_string()],
        ..Default::default()
    });
    let devices = vec![device];

    // Act
    validate_devices(&devices, &mut report);

    // Assert
    assert!(report.has_violations());
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.message.contains("must not contain a catch-all network"))
    );
}

#[test]
fn validate_identity_device_trusted_proxy_public_ip_warning() {
    // Arrange
    let mut report = ValidationReport::default();
    let device = DeviceSpec::Identity(IdentityDeviceSpec {
        enable: true,
        trusted_proxies: vec!["8.8.8.8/32".to_string()],
        ..Default::default()
    });
    let devices = vec![device];

    // Act
    validate_devices(&devices, &mut report);

    // Assert
    assert!(report.has_violations());
    assert!(report.warnings.iter().any(|w| {
        w.message
            .contains("should NOT contain a public IP range: 8.8.8.8/32")
    }))
}

#[test]
fn validate_identity_device_geoip_db_empty() {
    // Arrange
    let mut report = ValidationReport::default();
    let device = DeviceSpec::Identity(IdentityDeviceSpec {
        enable: true,
        enable_geoip: true,
        geoip_city_db: Some(PathBuf::from("")),
        ..Default::default()
    });
    let devices = vec![device];

    // Act
    validate_devices(&devices, &mut report);

    // Assert
    assert!(report.has_violations());
    let error_messages: Vec<String> = report.errors.iter().map(|e| e.message.clone()).collect();
    assert!(
        error_messages
            .iter()
            .any(|m| m.contains("geoip db path is empty"))
    );
}

#[test]
fn validate_identity_device_geoip_db_does_not_exist() {
    // Arrange
    let mut report = ValidationReport::default();
    let device = DeviceSpec::Identity(IdentityDeviceSpec {
        enable: true,
        enable_geoip: true,
        geoip_city_db: Some(PathBuf::from("/non/existent/geoip.db")),
        ..Default::default()
    });
    let devices = vec![device];

    // Act
    validate_devices(&devices, &mut report);

    // Assert
    assert!(report.has_violations());
    let error_messages: Vec<String> = report.errors.iter().map(|e| e.message.clone()).collect();
    assert!(
        error_messages
            .iter()
            .any(|m| m.contains("geoip db path does not exist"))
    );
}

#[test]
fn validate_identity_device_geoip_db_is_not_a_file() {
    let mut report = ValidationReport::default();
    let dir = tempfile::tempdir().unwrap();

    let device = DeviceSpec::Identity(IdentityDeviceSpec {
        enable: true,
        enable_geoip: true,
        geoip_city_db: Some(dir.path().to_path_buf()), // directory
        ..Default::default()
    });

    validate_devices(&[device], &mut report);

    assert!(
        report
            .errors
            .iter()
            .any(|e| e.message.contains("geoip db path is not a file"))
    );
}
