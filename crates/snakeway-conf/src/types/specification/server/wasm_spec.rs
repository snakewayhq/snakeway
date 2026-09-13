use confval::prelude::{Located, Report, Validate, range_constraint};
use serde::Serialize;

// wasmtime pre-reserves virtual address space proportional to the instance pool
// size multiplied by the per-instance memory ceiling, so both bounds are kept
// conservative to avoid an unbootable engine.
range_constraint!(MAX_CONCURRENT_EXECUTIONS, i64, min: 1, max: 8192);
range_constraint!(MAX_MEMORY_BYTES, i64, min: 1_048_576, max: 268_435_456, units: "bytes");

#[derive(Debug, Serialize, confval::Spec)]
#[confval(derive_default)]
pub struct WasmSpec {
    /// Maximum number of WASM device hook executions allowed to run at once.
    /// This sizes the wasmtime instance pool.
    /// Requests beyond this limit fail according to the device `fail_policy`.
    #[confval(default = 512, range = MAX_CONCURRENT_EXECUTIONS)]
    pub max_concurrent_executions: Located<i64>,

    /// Maximum linear memory, in bytes, that a single WASM device execution may use.
    #[confval(default = 67_108_864, range = MAX_MEMORY_BYTES)] // 64 MiB
    pub max_memory_bytes: Located<i64>,
}

impl Validate for WasmSpec {
    fn validate(&self, _report: &mut Report) {}
}
