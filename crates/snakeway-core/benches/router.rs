use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use snakeway_core::bench_api::RouteId;
use snakeway_core::bench_api::{RouteRuntime, Router};

fn build_router(n: usize) -> Router {
    let mut router = Router::new();
    for i in 0..n {
        let path = format!("/route/{i}");
        router
            .add_route(
                vec!["example.com".to_string()],
                &path,
                RouteRuntime::Service {
                    id: RouteId::service(&path, "upstream"),
                    upstream: "upstream".to_string(),
                    allow_websocket: false,
                    ws_max_connections: None,
                },
            )
            .unwrap();
    }
    router
}

fn bench_router_matching(c: &mut Criterion) {
    let mut group = c.benchmark_group("router_matching");

    for n in [1usize, 10, 100, 500, 1000] {
        let router = build_router(n);
        // Target the shortest path so the longest-prefix sort puts it last (worst-case scan).
        let target_path = "/route/0".to_string();

        group.bench_with_input(BenchmarkId::new("routes", n), &n, |b, _| {
            b.iter(|| router.match_route("example.com", &target_path).ok());
        });
    }

    group.finish();
}

criterion_group!(benches, bench_router_matching);
criterion_main!(benches);
