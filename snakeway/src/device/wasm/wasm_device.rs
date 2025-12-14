use anyhow::{anyhow, Result};
use wasmtime::{
    component::{Component, Linker}, Engine,
    Store,
};

use crate::ctx::{RequestCtx, ResponseCtx};
use crate::device::core::{result::DeviceResult, Device};
use http::{HeaderMap, StatusCode};
use wasmtime::component::ResourceTable;
use wasmtime_wasi::{p2::add_to_linker_sync, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

use crate::device::wasm::bindings::{
    exports::snakeway::device::policy::{Decision, Header, Request},
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

pub(crate) struct HostState {
    pub(crate) table: ResourceTable,
    pub(crate) wasi: WasiCtx,
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            table: &mut self.table,
            ctx: &mut self.wasi,
        }
    }
}

impl Device for WasmDevice {
    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        let mut linker = Linker::new(&self.engine);
        add_to_linker_sync(&mut linker).expect("failed to add WASI to linker");
        let wasi_ctx = WasiCtxBuilder::new().build();
        let mut store = Store::new(
            &self.engine,
            HostState {
                table: ResourceTable::new(),
                wasi: wasi_ctx,
            },
        );

        let instance = match Snakeway::instantiate(&mut store, &self.component, &linker) {
            Ok(i) => i,
            Err(e) => {
                log::error!("WASM instantiate failed: {e}");
                return DeviceResult::Continue;
            }
        };

        let req = Request {
            path: ctx.uri.path().to_string(),
            headers: ctx
                .headers
                .iter()
                .map(|(k, v)| Header {
                    name: k.to_string(),
                    value: v.to_str().unwrap_or("").to_string(),
                })
                .collect(),
        };

        let result = instance
            .snakeway_device_policy()
            .call_on_request(&mut store, &req)
            .map_err(|e| {
                log::error!("WASM device failed: {e}");
                DeviceResult::Continue
            })
            .expect("on_request failed");

        match result.decision {
            Decision::Block => {
                return DeviceResult::ShortCircuit(block_403());
            }
            Decision::Continue => {}
        }

        if let Some(patch) = result.patch {
            if let Some(new_path) = patch.set_path {
                ctx.uri = new_path.parse().unwrap();
            }

            for header in patch.set_headers {
                ctx.headers.insert(
                    header.name.parse::<http::HeaderName>().unwrap(),
                    header.value.parse().unwrap(),
                );
            }

            for name in patch.remove_headers {
                ctx.headers.remove(name.as_str());
            }
        }

        DeviceResult::Continue
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
