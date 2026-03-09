use crate::execution::ctx::RequestCtx;
use crate::execution::device::builtin::request_filter::RequestFilterDevice;
use crate::execution::device::core::Device;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use http::HeaderName;
use snakeway_conf::types::RequestFilterDeviceConfig;

/// Build a `RequestFilterDevice` with `n` deny-header rules.
///
/// This isolates the cost of iterating through the deny list on every request:
/// - `n = 0`   → header gate is a no-op (fast path)
/// - `n = 10`  → 10 `contains_key()` probes (within SmallVec inline capacity)
/// - `n = 100` → 100 `contains_key()` probes (SmallVec spills to heap)
fn device_with_deny_headers(n: usize) -> RequestFilterDevice {
    let deny_headers = (0..n)
        .map(|i| {
            HeaderName::from_bytes(format!("x-deny-{i:04}").as_bytes()).expect("valid header name")
        })
        .collect();

    RequestFilterDevice::from_config(RequestFilterDeviceConfig {
        deny_headers,
        max_header_bytes: usize::MAX,
        max_body_bytes: usize::MAX,
        max_suspicious_body_bytes: usize::MAX,
        ..Default::default()
    })
    .expect("valid config")
}

fn bench_request_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_filter");

    for n in [0usize, 10, 100] {
        let device = device_with_deny_headers(n);

        group.bench_with_input(BenchmarkId::new("deny_headers", n), &n, |b, _| {
            b.iter(|| {
                let mut ctx = RequestCtx::empty();
                device.on_request(&mut ctx)
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_request_filter);
criterion_main!(benches);
