use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, confval::Config)]
#[confval(lower_from = WasmSpec, validate)]
pub struct WasmConfig {
    /// Maximum concurrent WASM device hook executions (sizes the wasmtime pool).
    #[confval(lower(from = max_concurrent_executions, with = narrow::i64_to_u32))]
    pub max_concurrent_executions: u32,
    /// Per-execution linear memory ceiling, in bytes.
    #[confval(lower(from = max_memory_bytes, with = narrow::i64_to_usize))]
    pub max_memory_bytes: usize,
}

impl Default for WasmConfig {
    fn default() -> Self {
        // Mirrors the defaults in `WasmSpec`.
        // Used by CLI scaffolding that builds a throwaway engine outside the config pipeline.
        Self {
            max_concurrent_executions: 512,
            max_memory_bytes: 64 * 1024 * 1024,
        }
    }
}
