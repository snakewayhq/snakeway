pub mod builtin;
pub mod core;
pub(crate) mod wasm;

use crate::execution::device::core::Device;
use std::path::Path;
use std::sync::Arc;

pub fn load_wasm_device(device_file_path: &Path) -> anyhow::Result<Arc<dyn Device>> {
    use snakeway_conf::types::{WasmConfig, WasmDeviceConfig, WasmDeviceFailPolicy};
    let wasm_settings = WasmConfig::default();
    let wasm_engine = wasm::WasmEngine::new(&wasm_settings)?;
    let cfg = WasmDeviceConfig {
        name: "cli-test".to_string(),
        path: device_file_path.to_path_buf(),
        fail_policy: WasmDeviceFailPolicy::Open,
        timeout_ms: 5,
        ..Default::default()
    };
    let device = wasm::WasmDevice::load(
        Arc::clone(&wasm_engine.engine),
        &cfg,
        wasm_settings.max_memory_bytes,
    )?;
    let device: Arc<dyn Device> = Arc::new(device);
    // Leak the engine so the epoch ticker stays alive for the CLI command's lifetime.
    // This function is CLI scaffolding/workaround.
    // Production code uses DeviceRegistry which owns the WasmEngine properly.
    Box::leak(Box::new(wasm_engine));
    Ok(device)
}
