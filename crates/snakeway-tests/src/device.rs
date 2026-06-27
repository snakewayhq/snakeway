use crate::constants::TEST_DEVICE_PATH;
use confval::prelude::Located;
use snakeway_core::testing_api::conf::types::WasmDeviceSpec;
use std::collections::HashMap;
use std::path::PathBuf;

pub fn make_wasm_device(config: HashMap<String, String>) -> WasmDeviceSpec {
    WasmDeviceSpec {
        name: Located::detached("test-device".to_string()),
        enable: Located::detached(true),
        path: Located::detached(PathBuf::from(TEST_DEVICE_PATH)),
        fail_policy: Located::detached("open".to_string()),
        timeout_ms: Located::detached(100),
        config,
        ..Default::default()
    }
}

pub fn default_device() -> WasmDeviceSpec {
    make_wasm_device(HashMap::new())
}

pub fn device_with_mode(mode: &str) -> WasmDeviceSpec {
    make_wasm_device(HashMap::from([("mode".to_string(), mode.to_string())]))
}

/// A mode-driven device that only declares the given lifecycle hooks, the host skips the rest.
/// Used to verify the `hooks` allowlist.
pub fn device_with_mode_and_hooks(mode: &str, hooks: &[&str]) -> WasmDeviceSpec {
    let located_hooks = hooks
        .iter()
        .map(|h| Located::detached(h.to_string()))
        .collect();
    WasmDeviceSpec {
        hooks: Some(Located::detached(located_hooks)),
        ..device_with_mode(mode)
    }
}
