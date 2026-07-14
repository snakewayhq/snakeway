use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use snakeway_server::bench_api::{
    Device, IdentityDevice, IdentityDeviceConfig, RequestCtx, UaEngineKind,
};

/// Build an `IdentityDevice` with `n` trusted-proxy CIDR rules.
///
/// GeoIP and user-agent parsing are both disabled so the benchmark isolates
/// the overhead of IP trust-chain evaluation via the `resolve_client_ip` path.
///
/// Varying trusted_proxies count shows the cost of:
/// - Device construction (CIDR parsing at startup)
/// - `on_request` fast-path (no X-Forwarded-For header → direct connection)
fn device_with_trusted_proxies(n: usize) -> IdentityDevice {
    let trusted_proxies = (0..n).map(|i| format!("10.{i}.0.0/24")).collect();

    IdentityDevice::try_from(IdentityDeviceConfig {
        enable: true,
        trusted_proxies,
        max_x_forwarded_for_length: 10,
        enable_geoip: false,
        geoip_city_db: None,
        geoip_isp_db: None,
        geoip_connection_type_db: None,
        enable_user_agent: false,
        ua_engine: UaEngineKind::Woothee,
        ua_parser_regexes: None,
        max_user_agent_length: 512,
    })
    .expect("valid config")
}

fn bench_identity(c: &mut Criterion) {
    let mut group = c.benchmark_group("identity_device");

    for n in [0usize, 1, 10] {
        let device = device_with_trusted_proxies(n);

        group.bench_with_input(BenchmarkId::new("trusted_proxies", n), &n, |b, _| {
            b.iter(|| {
                let mut ctx = RequestCtx::empty();
                device.on_request(&mut ctx)
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_identity);
criterion_main!(benches);
