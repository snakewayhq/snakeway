pub(crate) mod builtin;
pub(crate) mod core;
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
    let device = wasm::wasm_device::WasmDevice::load(device_file_path)?;
    Ok(Arc::new(device))
}
