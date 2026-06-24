pub mod builtin;
pub mod core;
pub(crate) mod wasm;

use crate::execution::device::core::Device;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) fn load_wasm_device(device_file_path: &PathBuf) -> anyhow::Result<Arc<dyn Device>> {
    use snakeway_conf::types::WasmDeviceFailPolicy;
    let wasm_engine = wasm::WasmEngine::new()?;
    let device = wasm::WasmDevice::load(
        Arc::clone(&wasm_engine.engine),
        device_file_path,
        "cli-test".to_string(),
        WasmDeviceFailPolicy::Open,
        5,
        0,
        std::collections::HashMap::new(),
    )?;
    let device: Arc<dyn Device> = Arc::new(device);
    // Leak the engine so the epoch ticker stays alive for the CLI command's lifetime.
    // This function is CLI scaffolding/workaround.
    // Production code uses DeviceRegistry which owns the WasmEngine properly.
    Box::leak(Box::new(wasm_engine));
    Ok(device)
}
