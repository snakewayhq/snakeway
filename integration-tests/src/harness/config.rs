use snakeway_core::conf::RuntimeConfig;
use url::Url;

pub fn patch_ports(
    mut cfg: RuntimeConfig,
    listener_ports: &[u16],
    upstream_ports: &[u16],
) -> RuntimeConfig {
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

    // Patch upstream URLs (preserve scheme!)
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
        let mut url = Url::parse(&up.url).expect("invalid upstream URL in fixture");

        url.set_port(Some(upstream_ports[i]))
            .expect("failed to set upstream port");

        up.url = url.to_string();
    }

    cfg
}
