use snakeway_tests::harness::configdir::{AdminListener, ManualTls, ProcessConfig};
use snakeway_tests::harness::process::snakeway_bin;
use std::path::PathBuf;
use std::process::Command;

fn run_config_check(dir: &std::path::Path) -> Result<(), String> {
    let output = Command::new(snakeway_bin())
        .args(["config", "check"])
        .arg(dir)
        .output()
        .unwrap();
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn minimal_config(pid_file: PathBuf) -> ProcessConfig {
    ProcessConfig {
        listener_port: 18080,
        upstream_ports: vec![19001, 19002],
        enable_http2: false,
        tls_manual: None,
        admin: None,
        connection_filter_hcl: None,
        rate_limiting_hcl: None,
        server_fragment: None,
        pid_file: Some(pid_file),
        device_files: vec![(
            "10-structured-logging.hcl".to_string(),
            "structured_logging_device = {\n  enable = true\n  include_headers = false\n  allowed_headers = []\n  redacted_headers = []\n  level = \"info\"\n  include_identity = false\n  identity_fields = []\n}\n".to_string(),
        )],
        extra_ingress_files: vec![],
        drain_seconds: 5,
        force_timeout_seconds: 10,
    }
}

#[test]
fn process_config_template_passes_config_check() {
    // Arrange
    let dir = tempfile::tempdir().unwrap();
    let cfg = minimal_config(PathBuf::from("/tmp/snakeway-harness-check.pid"));
    let written = cfg.write_to(dir.path());

    // Act
    let result = run_config_check(&written);

    // Assert
    assert!(
        result.is_ok(),
        "generated config must pass config check: {result:?}"
    );
}

#[test]
fn process_config_full_shape_passes_config_check() {
    // Arrange
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = minimal_config(PathBuf::from("/tmp/snakeway-harness-full.pid"));
    cfg.listener_port = 18081;
    cfg.enable_http2 = true;
    cfg.tls_manual = Some(ManualTls::default());
    cfg.admin = Some(AdminListener::default());
    cfg.connection_filter_hcl = Some(
        "    cidr {\n      allow = [\"127.0.0.1/32\"]\n      deny = []\n    }\n    ip_family {\n      ipv4 = true\n      ipv6 = false\n    }\n    on_no_peer_addr = \"allow\"\n".to_string(),
    );
    cfg.rate_limiting_hcl =
        Some("    max_connections_per_second = 100\n    window_seconds = 1\n".to_string());
    cfg.server_fragment =
        Some("upgrade {\n    sock = \"/tmp/snakeway-harness-upgrade.sock\"\n  }".to_string());
    let written = cfg.write_to(dir.path());

    // Act
    let result = run_config_check(&written);

    // Assert
    assert!(
        result.is_ok(),
        "the TLS, admin, filter, and upgrade shapes must pass config check: {result:?}"
    );
}
