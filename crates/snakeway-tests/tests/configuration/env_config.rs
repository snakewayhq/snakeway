use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;

use snakeway_tests::constants::FIXTURES_CONFIG_DIR;

static BUILD_ONCE: Once = Once::new();

fn snakeway_bin() -> PathBuf {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bin = workspace_root.join("target/debug/snakeway");

    BUILD_ONCE.call_once(|| {
        let status = Command::new("cargo")
            .args(["build", "-p", "snakeway"])
            .current_dir(&workspace_root)
            .status()
            .expect("failed to run cargo build");
        assert!(status.success(), "cargo build -p snakeway failed");
    });

    assert!(
        bin.exists(),
        "snakeway binary not found at {}",
        bin.display()
    );
    bin
}

fn fixture_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURES_CONFIG_DIR)
        .join(name)
}

#[test]
fn config_check_uses_snakeway_config_env() {
    // Arrange
    let bin = snakeway_bin();
    let config_dir = fixture_dir("basic");

    // Act
    let output = Command::new(&bin)
        .args(["config", "check", "--quiet"])
        .env("SNAKEWAY_CONFIG", &config_dir)
        .env_remove("RUST_LOG")
        .output()
        .expect("failed to execute snakeway binary");

    // Assert
    assert!(
        output.status.success(),
        "config check should succeed when SNAKEWAY_CONFIG points to a valid fixture;\n\
         exit code: {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn config_check_cli_arg_overrides_snakeway_config_env() {
    // Arrange
    let bin = snakeway_bin();
    let valid_dir = fixture_dir("basic");
    let nonexistent_dir = "/tmp/snakeway-test-does-not-exist";

    // Act: env var points to a nonexistent dir, but CLI arg points to a valid one.
    let output = Command::new(&bin)
        .args(["config", "check", "--quiet"])
        .arg(valid_dir.as_os_str())
        .env("SNAKEWAY_CONFIG", nonexistent_dir)
        .env_remove("RUST_LOG")
        .output()
        .expect("failed to execute snakeway binary");

    // Assert
    assert!(
        output.status.success(),
        "CLI arg should override SNAKEWAY_CONFIG env var;\n\
         exit code: {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn config_check_fails_when_snakeway_config_env_points_to_missing_dir() {
    // Arrange
    let bin = snakeway_bin();

    // Act
    let output = Command::new(&bin)
        .args(["config", "check", "--quiet"])
        .env("SNAKEWAY_CONFIG", "/tmp/snakeway-test-does-not-exist")
        .env_remove("RUST_LOG")
        .output()
        .expect("failed to execute snakeway binary");

    // Assert
    assert!(
        !output.status.success(),
        "config check should fail when SNAKEWAY_CONFIG points to a nonexistent directory"
    );
}
