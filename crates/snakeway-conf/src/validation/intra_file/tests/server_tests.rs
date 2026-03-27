use crate::types::ServerSpec;
use crate::validation::{ValidateSpec, ValidationReport};
use std::path::PathBuf;

#[test]
fn validate_server_version_valid() {
    // Arrange
    let mut report = ValidationReport::default();
    let server = ServerSpec {
        version: 1,
        ..Default::default()
    };

    // Act
    server.validate(&server.origin, &mut report);

    // Assert
    assert!(!report.has_violations());
}

#[test]
fn validate_server_version_invalid() {
    // Arrange
    let mut report = ValidationReport::default();
    let server = ServerSpec {
        version: 2,
        ..Default::default()
    };

    // Act
    server.validate(&server.origin, &mut report);

    // Assert
    assert!(report.has_violations());
    assert!(
        report.errors[0]
            .message
            .contains("invalid config version: 2")
    );
}

#[test]
fn validate_server_valid_config() {
    // Arrange
    let mut report = ValidationReport::default();
    let server = ServerSpec {
        version: 1,
        threads: Some(4),
        ..Default::default()
    };

    // Act
    server.validate(&server.origin, &mut report);

    // Assert
    assert!(!report.has_violations());
}

#[test]
fn validate_server_pid_file_parent_dir_does_not_exist() {
    // Arrange
    let mut report = ValidationReport::default();
    let server = ServerSpec {
        pid_file: Some(PathBuf::from("/non/existent/path/snakeway.pid")),
        ..Default::default()
    };

    // Act
    server.validate(&server.origin, &mut report);

    // Assert
    assert!(report.has_violations());
    assert!(
        report.errors[0]
            .message
            .contains("pid file parent directory does not exist")
    );
}

#[test]
fn validate_server_ca_file_does_not_exist() {
    // Arrange
    let ca_file = PathBuf::from("/non/existent/ca.pem");
    let mut report = ValidationReport::default();
    let server = ServerSpec {
        ca_file: Some(ca_file.clone()),
        ..Default::default()
    };
    let expected = format!(
        "server CA file is invalid: file does not exist: {}",
        ca_file.to_string_lossy()
    );

    // Act
    server.validate(&server.origin, &mut report);

    // Assert
    assert!(report.has_violations());
    assert!(report.errors[0].message.contains(&expected));
}

#[test]
fn validate_server_threads_too_low() {
    // Arrange
    let mut report = ValidationReport::default();
    let server = ServerSpec {
        threads: Some(0),
        ..Default::default()
    };

    // Act
    server.validate(&server.origin, &mut report);

    // Assert
    assert!(report.has_violations());
    assert!(
        report.errors[0]
            .message
            .contains("invalid server.threads: 0")
    );
}

#[test]
fn validate_server_threads_too_high() {
    // Arrange
    let mut report = ValidationReport::default();
    let server = ServerSpec {
        threads: Some(1025),
        ..Default::default()
    };

    // Act
    server.validate(&server.origin, &mut report);

    // Assert
    assert!(report.has_violations());
    assert!(
        report.errors[0]
            .message
            .contains("invalid server.threads: 1025")
    );
}

#[test]
fn validate_server_pid_file_parent_is_not_a_dir() {
    // Arrange
    let mut report = ValidationReport::default();
    let dir = tempfile::tempdir().unwrap();

    // Create a file that will be used as the "parent"
    let fake_parent = dir.path().join("not_a_dir");
    std::fs::write(&fake_parent, "hello").unwrap();

    let server = ServerSpec {
        pid_file: Some(fake_parent.join("snakeway.pid")),
        ..Default::default()
    };

    // Act
    server.validate(&server.origin, &mut report);

    // Assert
    assert!(report.has_violations());
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.message.contains("pid file parent is not a directory"))
    );
}

#[test]
fn validate_server_ca_file_is_not_a_file() {
    // Arrange
    let dir = tempfile::tempdir().unwrap();
    let ca_file = PathBuf::from(dir.path());
    let expected = format!(
        "server CA file is invalid: file does not exist: {}",
        ca_file.to_string_lossy()
    );
    let mut report = ValidationReport::default();
    let server = ServerSpec {
        ca_file: Some(ca_file.clone()),
        ..Default::default()
    };

    // Act
    server.validate(&server.origin, &mut report);

    // Assert
    assert!(report.has_violations());
    assert!(report.errors.iter().any(|e| e.message.contains(&expected)));
}

#[test]
fn validate_server_valid_pid_and_ca_files() {
    // Arrange
    let mut report = ValidationReport::default();
    let dir = tempfile::tempdir().unwrap();

    let pid_dir = dir.path().join("pid");
    std::fs::create_dir(&pid_dir).unwrap();

    let ca_file = dir.path().join("ca.pem");
    std::fs::write(&ca_file, "dummy").unwrap();

    let server = ServerSpec {
        pid_file: Some(pid_dir.join("snakeway.pid")),
        ca_file: Some(ca_file),
        ..Default::default()
    };

    // Act
    server.validate(&server.origin, &mut report);

    // Assert
    assert!(!report.has_violations());
}
