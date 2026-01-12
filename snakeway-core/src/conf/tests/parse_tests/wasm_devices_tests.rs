use crate::conf::parse::parse_devices;
use crate::conf::types::DeviceSpec;
use std::fs;
use tempfile::tempdir;

#[test]
fn parse_wasm_device_array() {
    // Arrange
    let dir = tempdir().unwrap();
    let path = dir.path().join("wasm.hcl");

    fs::write(
        &path,
        r#"
wasm_devices = [
  { enable = false, path = "./a.wasm", config = {} },
  { enable = true,  path = "./b.wasm", config = {} }
]
"#,
    )
    .unwrap();

    // Act
    let devices = parse_devices(&path).unwrap();

    // Assert
    assert_eq!(devices.len(), 2);
    assert!(devices.iter().all(|d| matches!(d, DeviceSpec::Wasm(_))));
}

#[test]
fn parse_devices_empty_file_is_ok() {
    // Arrange
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty.hcl");

    fs::write(&path, "").unwrap();

    // Act
    let devices = parse_devices(&path).unwrap();

    // Assert
    assert!(devices.is_empty());
}
