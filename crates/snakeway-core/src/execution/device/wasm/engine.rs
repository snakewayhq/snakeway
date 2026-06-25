use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use wasmtime::{Config, Engine, InstanceAllocationStrategy, PoolingAllocationConfig};

pub(crate) const EPOCH_TICK_MS: u64 = 10;

pub(crate) struct WasmEngine {
    pub(crate) engine: Arc<Engine>,
    shutdown: Arc<AtomicBool>,
    ticker: Option<std::thread::JoinHandle<()>>,
}

impl WasmEngine {
    pub(crate) fn new() -> anyhow::Result<Self> {
        let mut pool = PoolingAllocationConfig::new();
        pool.total_component_instances(512);
        pool.max_component_instance_size(1 << 20);
        pool.max_memory_size(64 * 1024 * 1024);
        pool.total_memories(1024);
        pool.total_tables(1024);
        pool.total_core_instances(2048);
        pool.total_stacks(512);

        let mut config = Config::new();
        config.wasm_component_model(true);
        config.epoch_interruption(true);
        config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

        let engine = Arc::new(Engine::new(&config)?);
        let shutdown = Arc::new(AtomicBool::new(false));

        let ticker_engine = Arc::clone(&engine);
        let ticker_shutdown = Arc::clone(&shutdown);
        let ticker = std::thread::Builder::new()
            .name("wasm-epoch-ticker".to_string())
            .spawn(move || {
                while !ticker_shutdown.load(Ordering::Relaxed) {
                    std::thread::park_timeout(std::time::Duration::from_millis(EPOCH_TICK_MS));
                    ticker_engine.increment_epoch();
                }
            })?;

        Ok(Self {
            engine,
            shutdown,
            ticker: Some(ticker),
        })
    }
}

impl Drop for WasmEngine {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(t) = self.ticker.take() {
            t.thread().unpark();
            let _ = t.join();
        }
    }
}
