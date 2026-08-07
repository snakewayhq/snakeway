#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test helpers fail tests by panicking. The clippy.toml test carve-out does not reach them."
)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn snakeway() -> Command {
    Command::new(env!("CARGO_BIN_EXE_snakeway"))
}

fn init_minimal() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cfg = tmp.path().join("cfg");
    let out = snakeway()
        .args(["config", "init"])
        .arg(&cfg)
        .arg("--template=minimal")
        .output()
        .expect("run config init");
    assert!(
        out.status.success(),
        "config init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    (tmp, cfg)
}

fn dump(cfg: &Path, repr: &str, format: &str) -> String {
    let out = snakeway()
        .args(["config", "dump"])
        .arg(cfg)
        .args(["--repr", repr, "--format", format])
        .output()
        .expect("run config dump");
    assert!(
        out.status.success(),
        "config dump --repr {repr} --format {format} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn dump_spec_hcl_reflects_only_the_source() {
    // Arrange
    let (_tmp, cfg) = init_minimal();

    // Act
    let out = dump(&cfg, "spec", "hcl");

    // Assert
    assert!(out.contains("server {"), "out: {out}");
    assert!(out.contains("threads = 8"), "out: {out}");
    assert!(
        !out.contains("trusted_proxies"),
        "the spec representation must omit values the source did not write: {out}"
    );
}

#[test]
fn dump_populated_spec_hcl_fills_defaults() {
    // Arrange
    let (_tmp, cfg) = init_minimal();

    // Act
    let out = dump(&cfg, "populated-spec", "hcl");

    // Assert
    assert!(out.contains("trusted_proxies = []"), "out: {out}");
}

#[test]
fn dump_spec_json_omits_defaulted_server_blocks() {
    // Arrange
    let (_tmp, cfg) = init_minimal();

    // Act
    let out = dump(&cfg, "spec", "json");

    // Assert
    assert!(out.contains("\"threads\": 8"), "out: {out}");
    assert!(
        !out.contains("drain_seconds"),
        "the spec representation must omit defaulted server blocks: {out}"
    );
}

#[test]
fn dump_populated_spec_json_fills_defaulted_server_blocks() {
    // Arrange
    let (_tmp, cfg) = init_minimal();

    // Act
    let out = dump(&cfg, "populated-spec", "json");

    // Assert
    assert!(out.contains("drain_seconds"), "out: {out}");
}

#[test]
fn dump_runtime_hcl_emits_the_lowered_config() {
    // Arrange
    let (_tmp, cfg) = init_minimal();

    // Act
    let out = dump(&cfg, "runtime", "hcl");

    // Assert
    assert!(out.contains("server = {"), "out: {out}");
    assert!(out.contains("threads = 8"), "out: {out}");
    assert!(out.contains("pid_file"), "out: {out}");
}
