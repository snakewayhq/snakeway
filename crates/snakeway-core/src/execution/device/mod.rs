pub mod builtin;
pub mod core;
#[cfg(feature = "wasm")]
pub(crate) mod wasm;

use crate::execution::device::core::Device;
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(not(feature = "wasm"))]
pub(crate) fn load_wasm_device(_device_file_path: &PathBuf) -> anyhow::Result<Arc<dyn Device>> {
    Err(anyhow::anyhow!(
        "WASM devices are disabled. Rebuild with --features wasm"
    ))
}

#[cfg(feature = "wasm")]
pub(crate) fn load_wasm_device(device_file_path: &PathBuf) -> anyhow::Result<Arc<dyn Device>> {
    use snakeway_conf::types::WasmDeviceFailPolicy;
    let engine = wasm::create_wasm_engine()?;
    let device = wasm::WasmDevice::load(
        engine,
        device_file_path,
        "cli-test".to_string(),
        WasmDeviceFailPolicy::Open,
        5,
        0,
        std::collections::HashMap::new(),
    )?;
    Ok(Arc::new(device))
}
