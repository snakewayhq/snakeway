use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use snakeway_core::bench_api::{Device, DevicePipeline, RequestCtx};
use std::sync::Arc;

struct NoopDevice;

impl Device for NoopDevice {
    fn name(&self) -> &str {
        "noop"
    }
}

fn make_devices(n: usize) -> Vec<Arc<dyn Device>> {
    (0..n)
        .map(|_| Arc::new(NoopDevice) as Arc<dyn Device>)
        .collect()
}

fn bench_device_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("device_pipeline");

    for n in [0usize, 1, 5, 10] {
        let devices = make_devices(n);

        group.bench_with_input(BenchmarkId::new("devices", n), &n, |b, _| {
            b.iter(|| {
                let mut ctx = RequestCtx::empty();
                DevicePipeline::run_on_request(&devices, &mut ctx)
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_device_pipeline);
criterion_main!(benches);
