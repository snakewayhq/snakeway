use snakeway_core::conf::RuntimeConfig;
use url::Url;

use snakeway_core::conf::types::RouteConfig;
use std::path::PathBuf;

pub fn patch_runtime(cfg: &mut RuntimeConfig, listener_ports: &[u16], upstream_ports: &[u16]) {
    patch_ports(cfg, listener_ports, upstream_ports);
    patch_paths(cfg);
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

fn patch_ports(cfg: &mut RuntimeConfig, listener_ports: &[u16], upstream_ports: &[u16]) {
    assert_eq!(
        listener_ports.len(),
        cfg.listeners.len(),
        "invalid number of ports allocated for listeners {} {}",
        listener_ports.len(),
        cfg.listeners.len()
    );

    // Patch listener addresses.
    for (i, port) in listener_ports.iter().enumerate() {
        cfg.listeners.get_mut(i).unwrap().addr = format!("127.0.0.1:{port}");
    }
}
