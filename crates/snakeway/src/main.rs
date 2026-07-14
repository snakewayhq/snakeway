use snakeway_server::run;

// Only compiled for alloc-profiling builds, production and timing-only (`hotpath`) builds
// keep the system allocator.
#[cfg(feature = "hotpath-alloc")]
#[global_allocator]
static GLOBAL: hotpath::CountingAllocator = hotpath::CountingAllocator::new();

fn main() {
    run();
}
