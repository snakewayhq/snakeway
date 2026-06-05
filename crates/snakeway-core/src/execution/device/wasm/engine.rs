use std::sync::Arc;
use wasmtime::{Config, Engine};

pub(crate) fn create_wasm_engine() -> anyhow::Result<Arc<Engine>> {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.epoch_interruption(true);

    let engine = Engine::new(&config)?;
    let engine = Arc::new(engine);

    let ticker_engine = Arc::clone(&engine);
    std::thread::Builder::new()
        .name("wasm-epoch-ticker".to_string())
        .spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(1));
                ticker_engine.increment_epoch();
            }
        })?;

    Ok(engine)
}
