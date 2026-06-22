use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use wasmtime::{Config, Engine, InstanceAllocationStrategy, PoolingAllocationConfig};

pub(crate) const EPOCH_TICK_MS: u64 = 10;

pub(crate) struct WasmEngine {
    pub(crate) engine: Arc<Engine>,
    shutdown: Arc<AtomicBool>,
}

impl WasmEngine {
    pub(crate) fn new() -> anyhow::Result<Self> {
        let mut pool = PoolingAllocationConfig::new();
        pool.total_component_instances(64);
        pool.max_component_instance_size(1 << 20);
        pool.total_memories(128);
        pool.total_tables(128);
        pool.total_core_instances(256);

        let mut config = Config::new();
        config.wasm_component_model(true);
        config.epoch_interruption(true);
        config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

        let engine = Arc::new(Engine::new(&config)?);
        let shutdown = Arc::new(AtomicBool::new(false));

        let ticker_engine = Arc::clone(&engine);
        let ticker_shutdown = Arc::clone(&shutdown);
        std::thread::Builder::new()
            .name("wasm-epoch-ticker".to_string())
            .spawn(move || {
                while !ticker_shutdown.load(Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(EPOCH_TICK_MS));
                    ticker_engine.increment_epoch();
                }
            })?;

        Ok(Self { engine, shutdown })
    }
}

impl Drop for WasmEngine {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}
