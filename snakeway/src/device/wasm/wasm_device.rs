use anyhow::Result;
use wasmtime::{
    component::{Component, Linker},
    Engine, Store,
};

use http::{HeaderMap, StatusCode};

use crate::ctx::{RequestCtx, ResponseCtx};
use crate::device::core::{Device, result::DeviceResult};

use crate::device::wasm::bindings::{
    exports::snakeway::device::device::{Decision, Request},
    Snakeway,
};

/// WASM-backed Snakeway device (stateless, per-call execution)
pub struct WasmDevice {
    engine: Engine,
    component: Component,
}

impl WasmDevice {
    /// Load a WASM component from disk
    pub fn load(path: &str) -> Result<Self> {
        let engine = Engine::default();
        let component = Component::from_file(&engine, path)?;

        Ok(Self { engine, component })
    }
}

impl Device for WasmDevice {
    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        // Per-request execution state
        let mut store = Store::new(&self.engine, ());
        let linker = Linker::new(&self.engine);

        let instance = match Snakeway::instantiate(
            &mut store,
            &self.component,
            &linker,
        ) {
            Ok(i) => i,
            Err(e) => {
                log::error!("WASM instantiate failed: {e}");
                return DeviceResult::Continue;
            }
        };

        let req = Request {
            path: ctx.uri.path().to_string(),
        };

        match instance.snakeway_device_device().call_on_request(&mut store, &req) {
            Ok(Decision::Continue) => DeviceResult::Continue,
            Ok(Decision::Block) => DeviceResult::ShortCircuit(block_403()),
            Err(e) => {
                log::error!("WASM device error: {e}");
                DeviceResult::Continue
            }
        }
    }

    fn before_proxy(&self, _ctx: &mut RequestCtx) -> DeviceResult {
        DeviceResult::Continue
    }

    fn after_proxy(&self, _ctx: &mut ResponseCtx) -> DeviceResult {
        DeviceResult::Continue
    }

    fn on_response(&self, _ctx: &mut ResponseCtx) -> DeviceResult {
        DeviceResult::Continue
    }
}

/// Standard 403 response for blocked requests
fn block_403() -> ResponseCtx {
    ResponseCtx::new(
        StatusCode::FORBIDDEN,
        HeaderMap::new(),
        b"Blocked by device".to_vec(),
    )
}
