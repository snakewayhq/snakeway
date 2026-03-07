# Skill: benchmarking — Writing Criterion Microbenchmarks in `snakeway-core`

This skill documents how to write and run Criterion microbenchmarks for `snakeway-core`.
Follow these patterns precisely when adding new benchmarks.

## When to Use This Skill

Use this skill when you need to:
- Measure the performance of an internal `snakeway-core` component
- Detect performance regressions in a hot-path function
- Compare two implementations or configurations for speed

Do **not** create benchmarks for end-to-end proxy throughput. Use the existing k6 scripts
in `k6/` and the `benchmark-proxy` / `run-load-test` recipes in the Justfile instead.

---

## Benchmark Types and Locations

| Benchmark type | Location | Tool |
|---|---|---|
| Rust microbenchmarks | `snakeway-core/benches/` | Criterion |
| Proxy throughput / load tests | `k6/` | k6 |
| Manual load tests | Justfile recipes | wrk / hey |

---

## Criterion Template

Every benchmark file follows this structure:

```rust
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

fn bench_my_component(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_component");

    for n in [1usize, 10, 100] {
        // Arrange — build inputs outside the timed loop
        let input = build_input(n);

        group.bench_with_input(BenchmarkId::new("scenario", n), &n, |b, _| {
            b.iter(|| {
                // Act — only the hot path goes here
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
- Set `harness = false` in `Cargo.toml` for each bench target (already done).
- Build expensive inputs (Router, Device, HeaderMap) **outside** `b.iter()`.
- Only the code under measurement goes inside `b.iter()`.

---

## Adding a New Benchmark

### Step 1 — Identify the component

Benchmarks must target a single, well-defined public function or code path.
Good candidates (all synchronous, no I/O):

| Component | Function | File |
|---|---|---|
| Router matching | `Router::match_route()` | `route/router.rs` |
| Device pipeline dispatch | `DevicePipeline::run_on_request()` | `device/core/pipeline.rs` |
| Request filter device | `RequestFilterDevice::on_request()` | `device/builtin/request_filter.rs` |
| Identity device | `IdentityDevice::on_request()` | `device/builtin/identity.rs` |

> **Note on private functions**: internal helpers such as `normalize_headers()` and
> `resolve_client_ip()` are not re-exported publicly. Benchmark them indirectly through
> the device or ctx APIs that call them, or via a thin `#[cfg(test)]` / feature-gated
> re-export if direct measurement is essential.

### Step 2 — Name the file after the component

```
snakeway-core/benches/router.rs
snakeway-core/benches/device_pipeline.rs
snakeway-core/benches/request_filter.rs
snakeway-core/benches/identity.rs
```

### Step 3 — Register the bench target in `snakeway-core/Cargo.toml`

```toml
[[bench]]
name = "my_component"
harness = false
```

`criterion` is already in `[dev-dependencies]` and `[workspace.dependencies]`.

### Step 4 — Write the benchmark

Follow the template above. Use `BenchmarkId::new("dimension", value)` to parameterise
across scaling scenarios (e.g. 1/10/100 routes, 0/1/5/10 devices).

---

## Rules

1. **Deterministic** — benchmarks must produce the same result on every run.
2. **No network** — do not make HTTP requests or open sockets inside benchmarks.
3. **No external files** — do not read files from disk inside `b.iter()`.
   Pre-load any required data in the setup section outside the loop.
4. **No randomness** — use fixed, hard-coded inputs. If randomness is required,
   seed it outside `b.iter()` and pre-generate a fixed input slice.
5. **Single dimension** — each benchmark group should vary exactly one parameter
   (e.g. route count, device count, header count).
6. **Never modify production code** purely to make a function benchmarkable.
   Use the public API; if needed, add a narrow feature-gated re-export in a PR discussion.

---

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

---

## Interpreting Output

Criterion prints three timing columns per scenario:

```
router_matching/routes/1   time: [42.1 ns  42.5 ns  42.9 ns]
```

- Left / right bounds: 95 % confidence interval.
- Middle value: point estimate (use this for comparisons).
- `change: [-1.2% +0.3% +1.8%]` — shown on repeated runs; red means regression.

**Performance targets** (from the benchmarking guide):

| Metric | Target |
|---|---|
| Baseline proxy latency | < 1 ms |
| Device overhead per device | < 10 µs |
| TLS overhead | minimal |

---

## CI Policy

Benchmarks do **not** run on every CI build.

- Run manually during development: `just bench`
- Run in a nightly job or pre-release to catch regressions
- Do not add `cargo bench` to the standard `just test-everything` recipe
