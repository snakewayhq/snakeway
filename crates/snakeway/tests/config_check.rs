#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test helpers fail tests by panicking. The clippy.toml test carve-out does not reach them."
)]

use std::path::Path;
use std::process::Command;

fn snakeway() -> Command {
    Command::new(env!("CARGO_BIN_EXE_snakeway"))
}

fn init_minimal(dir: &Path) {
    let out = snakeway()
        .args(["config", "init"])
        .arg(dir)
        .arg("--template=minimal")
        .output()
        .expect("run config init");
    assert!(
        out.status.success(),
        "config init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn check_reports_semantic_failures_with_source_locations() {
    // Arrange
    let tmp = tempfile::tempdir().expect("tempdir");
    let cfg = tmp.path().join("cfg");
    init_minimal(&cfg);
    let device_dir = cfg.join("device.d");
    std::fs::create_dir_all(&device_dir).expect("device dir");
    std::fs::write(
        device_dir.join("rate_limit.hcl"),
        "request_rate_limiting_device {\n  enable = false\n  max_requests_per_second = 0\n  window_seconds = 5\n  paths = []\n}\n",
    )
    .expect("write invalid device file");

    // Act
    let out = snakeway()
        .args(["config", "check"])
        .arg(&cfg)
        .output()
        .expect("run config check");

    // Assert
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("max_requests_per_second must be at least 1"),
        "stderr must carry the semantic error: {stderr}"
    );
    assert!(
        stderr.contains("rate_limit.hcl:3:"),
        "semantic failures must render with source locations, not a debug dump: {stderr}"
    );
}

#[test]
fn check_succeeds_on_a_generated_template() {
    // Arrange
    let tmp = tempfile::tempdir().expect("tempdir");
    let cfg = tmp.path().join("cfg");
    init_minimal(&cfg);

    // Act
    let out = snakeway()
        .args(["config", "check"])
        .arg(&cfg)
        .output()
        .expect("run config check");

    // Assert
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("Config loaded successfully"));
}

fn init_one_service_with_two_upstreams(dir: &Path) {
    let out = snakeway()
        .args(["config", "init"])
        .arg(dir)
        .arg("--template=httpbin")
        .output()
        .expect("run config init");
    assert!(
        out.status.success(),
        "config init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::write(
        dir.join("ingress.d").join("httpbin.hcl"),
        r#"bind {
  interface = "loopback"
  port = 8080
  enable_http2 = false
}

services {
  name = "httpbin"
  load_balancing_strategy = "failover"

  routes {
    hosts = ["*"]
    path = "/get"
    enable_websocket = false
  }

  upstreams {
    weight = 1
    endpoint {
      host = "127.0.0.1"
      port = 4000
    }
  }

  upstreams {
    weight = 1
    endpoint {
      host = "127.0.0.1"
      port = 4001
    }
  }
}
"#,
    )
    .expect("write ingress file");
}

#[test]
fn check_counts_upstreams_separately_from_services() {
    // Arrange
    let tmp = tempfile::tempdir().expect("tempdir");
    let cfg = tmp.path().join("cfg");
    init_one_service_with_two_upstreams(&cfg);

    // Act
    let out = snakeway()
        .args(["config", "check"])
        .arg(&cfg)
        .output()
        .expect("run config check");

    // Assert
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        [
            "Config loaded successfully",
            "1 routes",
            "1 services",
            "2 upstreams",
            "1 devices enabled",
        ]
    );
}

#[test]
fn check_json_counts_upstreams_separately_from_services() {
    // Arrange
    let tmp = tempfile::tempdir().expect("tempdir");
    let cfg = tmp.path().join("cfg");
    init_one_service_with_two_upstreams(&cfg);

    // Act
    let out = snakeway()
        .args(["config", "check", "--format", "json"])
        .arg(&cfg)
        .output()
        .expect("run config check");

    // Assert
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(
        report,
        serde_json::json!({
            "status": "success",
            "routes": 1,
            "services": 1,
            "upstreams": 2,
            "devices_enabled": 1,
        })
    );
}
