use snakeway_conf::types::WasmConfig;
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
    pub(crate) fn new(config: &WasmConfig) -> anyhow::Result<Self> {
        // `max_concurrent_executions` sizes the component-instance pool. The other
        // pool limits are derived from it at the ratios wasmtime needs so that the
        // component-instance count stays the real ceiling rather than one of the
        // secondary limits.
        let executions = config.max_concurrent_executions;
        let mut pool = PoolingAllocationConfig::new();
        pool.total_component_instances(executions);
        pool.max_component_instance_size(1 << 20);
        pool.max_memory_size(config.max_memory_bytes);
        pool.total_memories(2 * executions);
        pool.total_tables(2 * executions);
        pool.total_core_instances(4 * executions);
        pool.total_stacks(executions);

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
