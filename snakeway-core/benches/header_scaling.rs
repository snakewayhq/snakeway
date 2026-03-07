use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use http::HeaderMap;
use snakeway_core::ctx::normalization::{ProtocolNormalizationMode, normalize_headers};

/// Build a `HeaderMap` with `n` synthetic headers.
///
/// Names follow the pattern `x-h-0000`, `x-h-0001`, … to stay valid HTTP
/// header names across all N. Values are a fixed ASCII string.
fn build_headers(n: usize) -> HeaderMap {
    let mut map = HeaderMap::with_capacity(n);
    for i in 0..n {
        let name = http::HeaderName::from_bytes(format!("x-h-{i:04}").as_bytes())
            .expect("valid header name");
        map.insert(name, http::HeaderValue::from_static("value"));
    }
    map
}

fn bench_header_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("header_scaling");

    for n in [1usize, 10, 100, 1000] {
        let headers = build_headers(n);

        group.bench_with_input(BenchmarkId::new("http1", n), &n, |b, _| {
            b.iter(|| normalize_headers(&headers, &ProtocolNormalizationMode::Http1));
        });

        group.bench_with_input(BenchmarkId::new("http2", n), &n, |b, _| {
            b.iter(|| normalize_headers(&headers, &ProtocolNormalizationMode::Http2));
        });
    }

    group.finish();
}

criterion_group!(benches, bench_header_scaling);
criterion_main!(benches);
