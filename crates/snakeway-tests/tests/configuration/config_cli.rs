use snakeway_core::testing_api::conf::load_config;
use snakeway_tests::constants::FIXTURES_CONFIG_DIR;

/// Loading the `basic` fixture directory must succeed without validation
/// errors. This exercises the full config pipeline: file discovery, HCL
/// parsing, spec validation, and lowering to runtime config.
#[test]
fn valid_fixture_loads_without_violations() {
    // Arrange
    let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURES_CONFIG_DIR)
        .join("basic");

    // Act
    let result = load_config(&fixture_dir);

    // Assert
    let validated = result.expect("valid fixture should load successfully");
    assert!(
        !validated.validation_report.has_violations(),
        "valid fixture should produce no validation violations"
    );
}

/// Loading a config directory that does not exist must return an error,
/// not panic or silently produce an empty config.
#[test]
fn missing_config_directory_returns_error() {
    // Arrange
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let nonexistent = tmp.path().join("does-not-exist");

    // Act
    let result = load_config(&nonexistent);

    // Assert
    assert!(
        result.is_err(),
        "loading a nonexistent config directory must return an error"
    );
}

/// A config file with invalid HCL syntax must produce a parse error,
/// not a panic.
#[test]
fn invalid_hcl_syntax_returns_parse_error() {
    // Arrange
    let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURES_CONFIG_DIR)
        .join("invalid_hcl");

    // Act
    let result = load_config(&fixture_dir);

    // Assert
    assert!(
        result.is_err(),
        "invalid HCL syntax must return a parse error"
    );
}

/// A config that parses but has semantic errors (e.g., missing required
/// fields) must produce validation violations rather than silently
/// succeeding.
#[test]
fn semantically_invalid_config_reports_violations() {
    // Arrange
    let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURES_CONFIG_DIR)
        .join("invalid_semantic");

    // Act
    let result = load_config(&fixture_dir);

    // Assert
    let validated = result.expect("semantically invalid config should still load (not hard-fail)");
    assert!(
        validated.validation_report.has_violations(),
        "invalid version should produce validation violations"
    );
    assert!(
        validated
            .validation_report
            .errors
            .iter()
            .any(|e| e.message.contains("invalid config version")),
        "should report invalid config version"
    );
}

/// A successfully loaded config must be serializable to JSON without
/// error. This is the foundation of the `snakeway config dump` command.
#[test]
fn loaded_config_serializes_to_json() {
    // Arrange
    let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURES_CONFIG_DIR)
        .join("basic");
    let validated = load_config(&fixture_dir).expect("fixture should load");

    // Act
    let json = serde_json::to_string_pretty(&validated.config);

    // Assert
    let json_str = json.expect("config must serialize to JSON");
    assert!(!json_str.is_empty(), "JSON output must not be empty");

    // Verify it's valid JSON by parsing it back
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("serialized JSON must parse back");
    assert!(
        parsed.is_object(),
        "config JSON must be an object at the top level"
    );
}

/// An empty directory (no snakeway.hcl) must return an error, not
/// silently produce an empty config.
#[test]
fn empty_config_directory_returns_error() {
    // Arrange
    let tmp = tempfile::tempdir().expect("failed to create temp dir");

    // Act
    let result = load_config(tmp.path());

    // Assert
    assert!(
        result.is_err(),
        "loading an empty config directory must return an error"
    );
}

/// A config that references a nonexistent CA file path must produce a
/// validation error rather than silently accepting the bad path.
#[test]
fn nonexistent_ca_file_produces_validation_error() {
    // Arrange
    use snakeway_core::testing_api::conf::types::ServerSpec;
    use snakeway_tests::conf::ConfigBuilder;
    use std::path::PathBuf;

    let validated = ConfigBuilder::default()
        .with_server_spec(ServerSpec {
            version: 1,
            threads: Some(1),
            ca_file: Some(PathBuf::from("/nonexistent/path/to/ca-cert.pem")),
            ..Default::default()
        })
        .with_http_ingress()
        .try_build();

    // Assert
    assert!(
        validated.validation_report.has_violations(),
        "nonexistent CA file path should produce validation errors"
    );
    assert!(
        validated
            .validation_report
            .errors
            .iter()
            .any(|e| e.message.contains("CA file")),
        "should report CA file error; got: {:?}",
        validated.validation_report.errors
    );
}
