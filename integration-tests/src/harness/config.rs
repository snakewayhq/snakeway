use snakeway_core::conf::RuntimeConfig;

pub fn patch_ports(
    mut cfg: RuntimeConfig,
    listen_port: u16,
    upstream_ports: &[u16],
) -> RuntimeConfig {
    // Patch listen
    cfg.listeners[0].addr = format!("127.0.0.1:{listen_port}");

    // Patch the upstream URLs for the "api" service (or whichever service your route targets)
    let svc = cfg
        .services
        .get_mut("api")
        .expect("fixture missing services.api");

    assert!(
        svc.upstream.len() <= upstream_ports.len(),
        "fixture defines {} upstreams but only {} ports allocated",
        svc.upstream.len(),
        upstream_ports.len()
    );

    for (i, up) in svc.upstream.iter_mut().enumerate() {
        up.url = format!("http://127.0.0.1:{}", upstream_ports[i]);
    }

    // todo to pin strategy for tests:
    // svc.strategy = Strategy::RoundRobin;

    cfg
}
