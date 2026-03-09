use crate::constants::{ACME_ORDERS_DIR, CERT_PEBBLE_CA_PEM, TEST_HOST};
use snakeway_core::testing_api::conf::types::RuntimeConfig;
use url::Url;

use snakeway_core::testing_api::conf::types::RouteConfig;
use std::path::PathBuf;

pub fn patch_runtime(cfg: &mut RuntimeConfig, listener_ports: &[u16], upstream_ports: &[u16]) {
    patch_listener_ports(cfg, listener_ports);
    patch_upstream_ports(cfg, upstream_ports);
    patch_paths(cfg);
    patch_acme_paths(cfg);
}

fn patch_paths(cfg: &mut RuntimeConfig) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().expect("expected workspace root");
    let fixtures_root = repo_root.join("integration-tests/fixtures/public");
    for route_cfg in &mut cfg.routes {
        if let RouteConfig::Static(route) = route_cfg {
            route.file_dir = fixtures_root.clone();
        }
    }
}

fn patch_listener_ports(cfg: &mut RuntimeConfig, listener_ports: &[u16]) {
    // Redirect listeners use fixed ports (e.g. 5002 for Pebble HTTP-01 challenges)
    // and must not be reassigned.
    let non_redirect_count = cfg
        .listeners
        .iter()
        .filter(|l| l.redirect.is_none())
        .count();
    assert_eq!(
        listener_ports.len(),
        non_redirect_count,
        "invalid number of ports allocated for listeners {} {}",
        listener_ports.len(),
        non_redirect_count
    );

    // Patch only non-redirect listener addresses.
    let mut port_iter = listener_ports.iter();
    for listener in cfg.listeners.iter_mut() {
        if listener.redirect.is_some() {
            continue;
        }
        if let Some(port) = port_iter.next() {
            listener.addr = format!("{TEST_HOST}:{port}");
        }
    }
}

fn patch_acme_paths(cfg: &mut RuntimeConfig) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let Some(tls_auto) = cfg.server.tls_automation.as_mut() else {
        return;
    };
    // Absolutize the ACME data dir so the test process can find it regardless of cwd.
    tls_auto.acme.data_dir = manifest_dir.join(ACME_ORDERS_DIR);
    // Absolutize the Pebble CA file path.
    if tls_auto.acme.ca_file.is_some() {
        tls_auto.acme.ca_file = Some(manifest_dir.join(CERT_PEBBLE_CA_PEM));
    }
}

fn patch_upstream_ports(cfg: &mut RuntimeConfig, upstream_ports: &[u16]) {
    if upstream_ports.is_empty() {
        return;
    }
    // Patch upstream URLs (preserve scheme)
    let svc = cfg
        .services
        .get_mut("127.0.0.1:8080-service")
        .expect("service not found");

    assert!(
        svc.tcp_upstreams.len() <= upstream_ports.len(),
        "fixture defines {} upstreams but only {} ports allocated",
        svc.tcp_upstreams.len(),
        upstream_ports.len()
    );

    for (i, up) in svc.tcp_upstreams.iter_mut().enumerate() {
        let mut url = Url::parse(&up.url).expect("invalid upstream URL in fixture");

        url.set_port(Some(upstream_ports[i]))
            .expect("failed to set upstream port");

        up.url = url.to_string();
    }
}
