use anyhow::Result;
use wasmtime::{
    component::{Component, Linker}, Engine,
    Store,
};

use http::{HeaderMap, HeaderName, StatusCode};
use wasmtime::component::ResourceTable;
use wasmtime_wasi::{p2::add_to_linker_sync, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

use crate::ctx::{RequestCtx, ResponseCtx};
use crate::device::core::{result::DeviceResult, Device};

use crate::device::wasm::bindings::{
    exports::snakeway::device::policy::{Decision, Header, Request, RequestPatch},
    Snakeway,
};

/// WASM-backed Snakeway device (stateless, per-call execution)
pub struct WasmDevice {
    engine: Engine,
    component: Component,
}

impl WasmDevice {
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

        let mut store = Store::new(
            &self.engine,
            HostState {
                table: ResourceTable::new(),
                wasi: WasiCtxBuilder::new().build(),
            },
        );

        let instance = match Snakeway::instantiate(&mut store, &self.component, &linker) {
            Ok(i) => i,
            Err(e) => {
                tracing::error!("WASM instantiate failed: {e}");
                return DeviceResult::Continue;
            }
        };

        // Build request snapshot for WASM
        let req = Request {
            original_path: ctx.original_uri.path().to_string(),
            route_path: ctx.route_path.clone(),
            headers: ctx
                .headers
                .iter()
                .map(|(k, v)| Header {
                    name: k.to_string(),
                    value: v.to_str().unwrap_or("").to_string(),
                })
                .collect(),
        };

        let result = match instance
            .snakeway_device_policy()
            .call_on_request(&mut store, &req)
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("WASM on_request failed: {e}");
                return DeviceResult::Continue;
            }
        };

        // Enforce decision
        if matches!(result.decision, Decision::Block) {
            return DeviceResult::Respond(block_403());
        }

        // Apply explicit patch intent
        if let Some(RequestPatch {
            set_route_path,
            set_upstream_path,
            set_headers,
            remove_headers,
        }) = result.patch
        {
            if let Some(path) = set_route_path {
                ctx.route_path = path;
            }

            if let Some(path) = set_upstream_path {
                ctx.upstream_path = Some(path);
            }

            for header in set_headers {
                if let (Ok(name), Ok(value)) = (header.name.parse::<HeaderName>(), header.value.parse()) {
                    ctx.headers.insert(name, value);
                }
            }

            for name in remove_headers {
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
