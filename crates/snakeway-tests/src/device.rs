use crate::constants::{TEST_DEVICE_PATH, TEST_JWT_DEVICE_PATH};
use confval::prelude::Located;
use snakeway::testing_api::conf::types::WasmDeviceSpec;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

fn to_spec_map(config: HashMap<String, String>) -> BTreeMap<String, Located<String>> {
    config
        .into_iter()
        .map(|(key, value)| (key, Located::detached(value)))
        .collect()
}

pub fn make_wasm_device(config: HashMap<String, String>) -> WasmDeviceSpec {
    let config = to_spec_map(config);
    WasmDeviceSpec {
        name: Located::detached("test-device".to_string()),
        enable: Located::detached(true),
        path: Located::detached(PathBuf::from(TEST_DEVICE_PATH)),
        fail_policy: Located::detached("open".to_string()),
        timeout_milliseconds: Located::detached(100),
        config,
        ..Default::default()
    }
}

/// The real JWT auth device (`snakeway-jwt-auth-device`) built as a fixture.
/// Uses fail_policy "closed" so an auth device that traps rejects rather than
/// passing the request through.
pub fn make_jwt_device(config: HashMap<String, String>) -> WasmDeviceSpec {
    let config = to_spec_map(config);
    WasmDeviceSpec {
        name: Located::detached("jwt-auth".to_string()),
        enable: Located::detached(true),
        path: Located::detached(PathBuf::from(TEST_JWT_DEVICE_PATH)),
        fail_policy: Located::detached("closed".to_string()),
        timeout_milliseconds: Located::detached(100),
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
