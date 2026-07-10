---
title: Benchmarks
---

This page documents how to write and run Criterion microbenchmarks for `snakeway-core`.
Follow these patterns precisely when adding new benchmarks.

## When to Write a Microbenchmark

Write a Criterion microbenchmark when you need to:

- Measure the performance of an internal `snakeway-core` component.
- Detect performance regressions in a hot-path function.
- Compare two implementations or configurations for speed.

Do **not** create microbenchmarks for end-to-end proxy throughput.
Use the existing k6 scripts in `k6/` and the `benchmark-proxy` and `run-load-test` recipes in the Justfile instead.

| Benchmark type                | Location                        | Tool      |
|-------------------------------|---------------------------------|-----------|
| Rust microbenchmarks          | `crates/snakeway-core/benches/` | Criterion |
| Proxy throughput / load tests | `k6/`                           | k6        |
| Manual load tests             | Justfile recipes                | wrk / hey |

## Criterion Template

Every benchmark file follows this structure:

```rust
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

fn bench_my_component(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_component");

    for n in [1usize, 10, 100] {
        // Arrange: build inputs outside the timed loop
        let input = build_input(n);

        group.bench_with_input(BenchmarkId::new("scenario", n), &n, |b, _| {
            b.iter(|| {
                // Act: only the hot path goes here
                my_component_function(&input)
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_my_component);
criterion_main!(benches);
```

Key rules:

- `criterion_group!` and `criterion_main!` are **required** in every file.
- Set `harness = false` in `Cargo.toml` for each bench target (already done for existing targets).
- Build expensive inputs (Router, Device, HeaderMap) **outside** `b.iter()`.
- Only the code under measurement goes inside `b.iter()`.

## Adding a New Benchmark

### Step 1: Identify the component

Benchmarks must target a single, well-defined public function or code path.
Good candidates are synchronous and perform no I/O:

| Component                | Function                          | File                              |
|--------------------------|-----------------------------------|-----------------------------------|
| Router matching          | `Router::match_route()`           | `route/router.rs`                 |
| Device pipeline dispatch | `DevicePipeline::run_on_request()`| `device/core/pipeline.rs`         |
| Header normalization     | `normalize_headers()`             | `ctx/normalization`               |
| Request filter device    | `RequestFilterDevice::on_request()`| `device/builtin/request_filter.rs`|
| Identity device          | `IdentityDevice::on_request()`    | `device/builtin/identity.rs`      |

:::note
`normalize_headers()` is accessible via the public `snakeway_core::ctx::normalization` re-export.
Functions still private to the crate (for example `resolve_client_ip` in `net::client_ip`) should be benchmarked indirectly through the device or ctx APIs that call them.
:::

### Step 2: Name the file after the component

The existing bench targets follow this pattern:

```
crates/snakeway-core/benches/router.rs
crates/snakeway-core/benches/device_pipeline.rs
crates/snakeway-core/benches/request_filter.rs
crates/snakeway-core/benches/identity.rs
crates/snakeway-core/benches/header_scaling.rs
```

### Step 3: Register the bench target in crates/snakeway-core/Cargo.toml

```toml
[[bench]]
name = "my_component"
harness = false
```

`criterion` is already in `[dev-dependencies]` and `[workspace.dependencies]`.

### Step 4: Write the benchmark

Follow the template above.
Use `BenchmarkId::new("dimension", value)` to parameterize across scaling scenarios (for example 1/10/100 routes, or 0/1/5/10 devices).

## Rules

1. **Deterministic.** Benchmarks must produce the same result on every run.
2. **No network.** Do not make HTTP requests or open sockets inside benchmarks.
3. **No external files.** Do not read files from disk inside `b.iter()`.
   Pre-load any required data in the setup section outside the loop.
4. **No randomness.** Use fixed, hard-coded inputs.
   If randomness is required, seed it outside `b.iter()` and pre-generate a fixed input slice.
5. **Single dimension.** Each benchmark group should vary exactly one parameter (for example route count, device count, or header count).
6. **Never modify production code** purely to make a function benchmarkable.
   Use the public API.
   If needed, propose a narrow feature-gated re-export in a PR discussion.

## Running Benchmarks

```bash
# Run all snakeway-core microbenchmarks
just bench

# Or directly
cargo bench -p snakeway-core

# Run a single benchmark by name
cargo bench -p snakeway-core --bench router

# Filter to a specific scenario
cargo bench -p snakeway-core --bench router -- router_matching/routes/100
```

HTML reports are written to `target/criterion/report/index.html` after each run.

## Interpreting Output

Criterion prints three timing columns per scenario:

```
router_matching/routes/1   time: [42.1 ns  42.5 ns  42.9 ns]
```

- The left and right bounds are the 95 percent confidence interval.
- The middle value is the point estimate.
  Use this for comparisons.
- A `change: [-1.2% +0.3% +1.8%]` line is shown on repeated runs.
  Red means regression.

**Performance targets:**

| Metric                     | Target      |
|----------------------------|-------------|
| Baseline proxy latency     | < 1 ms      |
| Device overhead per device | < 10 µs     |
| TLS overhead               | minimal     |

## CI Policy

Benchmarks do **not** run on every CI build.

- Run manually during development with `just bench`.
- `just test-everything` runs the benchmarks as its final step, so a full local pre-PR run exercises them.
- Run in a nightly job or pre-release to catch regressions.
