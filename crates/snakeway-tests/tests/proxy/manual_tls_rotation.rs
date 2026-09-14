use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::tls::TlsInfo;
use snakeway::testing_api::ControlPlaneServer;
use snakeway::testing_api::conf::load_config;
use snakeway_tests::constants::{ADMIN_TOKEN_FILE, FIXTURES_CONFIG_DIR, TEST_HOST};
use snakeway_tests::harness::server::{admin_client, free_port, wait_for_listener};
use std::path::Path;
use std::time::{Duration, Instant};

/// If you replace a manual certificate and key at the same paths and trigger a
/// reload, the next TLS handshake on that listener presents the new certificate.
#[test]
fn manual_tls_reload_presents_replaced_certificate() {
    // Arrange
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let cert_path = temp_dir.path().join("traffic.pem");
    let key_path = temp_dir.path().join("traffic.key");
    let first = write_self_signed_pair(&cert_path, &key_path);
    let listener_addr = format!("127.0.0.1:{}", free_port());
    let admin_addr = format!("127.0.0.1:{}", free_port());
    write_config_dir(
        temp_dir.path(),
        &listener_addr,
        &admin_addr,
        &cert_path,
        &key_path,
    );

    let validated = load_config(temp_dir.path()).expect("failed to load config");
    let server =
        ControlPlaneServer::build(Some(temp_dir.path().to_path_buf()), validated.config, false)
            .expect("failed to build server");
    let _running = server.run_background();
    wait_for_listener(&listener_addr);
    wait_for_listener(&admin_addr);
    assert_eq!(presented_certificate(&listener_addr), first);

    let second = write_self_signed_pair(&cert_path, &key_path);
    assert_ne!(first, second, "the replacement certificate must differ");

    // Act
    let reload = admin_client()
        .post(format!("https://{admin_addr}/admin/reload"))
        .send()
        .expect("reload request failed");

    // Assert
    assert_eq!(reload.status(), StatusCode::OK);
    assert_eq!(
        wait_for_presented_certificate(&listener_addr, &second),
        second
    );
}

fn write_self_signed_pair(cert_path: &Path, key_path: &Path) -> Vec<u8> {
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".into()])
        .expect("failed to generate self-signed cert");
    std::fs::write(cert_path, generated.cert.pem()).expect("failed to write cert");
    std::fs::write(key_path, generated.signing_key.serialize_pem()).expect("failed to write key");
    generated.cert.der().to_vec()
}

fn write_config_dir(
    dir: &Path,
    listener_addr: &str,
    admin_addr: &str,
    cert_path: &Path,
    key_path: &Path,
) {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURES_CONFIG_DIR)
        .join("basic");
    let cert_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("certs");
    let token_file = Path::new(env!("CARGO_MANIFEST_DIR")).join(ADMIN_TOKEN_FILE);
    let upstream_port = snakeway_tests::harness::upstream::start_http_upstream();
    let (listener_host, listener_port) = split_addr(listener_addr);
    let (admin_host, admin_port) = split_addr(admin_addr);

    std::fs::copy(fixture_dir.join("snakeway.hcl"), dir.join("snakeway.hcl")).unwrap();

    let device_dst = dir.join("device.d");
    std::fs::create_dir_all(&device_dst).unwrap();
    for entry in std::fs::read_dir(fixture_dir.join("device.d")).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), device_dst.join(entry.file_name())).unwrap();
    }

    let ingress_dst = dir.join("ingress.d");
    std::fs::create_dir_all(&ingress_dst).unwrap();

    let ingress_hcl = format!(
        r#"bind = {{
  interface    = "{listener_host}"
  port         = {listener_port}
  enable_http2 = false
  tls = {{
    mode = "manual"
    cert = "{cert}"
    key  = "{key}"
  }}
}}

services = [
  {{
    name = "api"
    routes = [
      {{
        hosts = ["{TEST_HOST}"]
        path = "/api"
      }}
    ]

    upstreams = [
      {{
        weight = 1
        endpoint = {{ host = "127.0.0.1", port = {upstream_port} }}
      }}
    ]
  }}
]
"#,
        cert = cert_path.display(),
        key = key_path.display(),
    );
    std::fs::write(ingress_dst.join("api.hcl"), ingress_hcl).unwrap();

    let admin_hcl = format!(
        r#"bind_admin = {{
  interface = "{admin_host}"
  port      = {admin_port}
  tls = {{
    mode = "manual"
    cert = "{cert}"
    key  = "{key}"
  }}
  auth = {{
    bearer = {{
      token_file = "{token_file}"
    }}
  }}
}}
"#,
        cert = cert_dir.join("server.pem").display(),
        key = cert_dir.join("server.key").display(),
        token_file = token_file.display(),
    );
    std::fs::write(ingress_dst.join("admin.hcl"), admin_hcl).unwrap();
}

fn split_addr(addr: &str) -> (&str, &str) {
    addr.rsplit_once(':').expect("address must be host:port")
}

fn presented_certificate(addr: &str) -> Vec<u8> {
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .tls_info(true)
        .timeout(Duration::from_secs(2))
        .build()
        .expect("failed to build client");
    let response = client
        .get(format!("https://{addr}/"))
        .send()
        .expect("request failed");
    response
        .extensions()
        .get::<TlsInfo>()
        .and_then(TlsInfo::peer_certificate)
        .expect("response has no peer certificate")
        .to_vec()
}

/// A reload runs on the control plane after the admin API returns, so poll
/// until the expected certificate appears or the deadline passes.
fn wait_for_presented_certificate(addr: &str, expected: &[u8]) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let presented = presented_certificate(addr);
        if presented == expected || Instant::now() >= deadline {
            return presented;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
