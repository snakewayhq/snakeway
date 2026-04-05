use anyhow::Result;
use bytes::Bytes;
use std::path::PathBuf;
use wasmtime::{
    Engine, Store,
    component::{Component, Linker},
};

use http::{HeaderMap, HeaderName, StatusCode};
use wasmtime::component::ResourceTable;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView, p2::add_to_linker_sync};

use crate::execution::ctx::{RequestCtx, RequestId, ResponseCtx};
use crate::execution::device::core::{Device, DeviceResult};

use crate::execution::device::wasm::bindings::{
    Snakeway,
    exports::snakeway::device::policy::{BodyChunk, Decision, Header, Request, RequestPatch},
};

pub(crate) struct WasmDevice {
    engine: Engine,
    component: Component,
}

impl WasmDevice {
    pub(crate) fn load(path: &PathBuf) -> Result<Self> {
        let engine = Engine::default();
        let component = Component::from_file(&engine, path)?;
        Ok(Self { engine, component })
    }

    /// Execute a closure with a freshly instantiated WASM component.
    fn with_instance<F>(&self, f: F) -> Option<DeviceResult>
    where
        F: FnOnce(&mut Store<HostState>, &Snakeway) -> Result<DeviceResult>,
    {
        let mut linker = Linker::new(&self.engine);
        add_to_linker_sync(&mut linker).ok()?;

        let mut store = Store::new(
            &self.engine,
            HostState {
                table: ResourceTable::new(),
                wasi: WasiCtxBuilder::new().build(),
            },
        );

        let instance = Snakeway::instantiate(&mut store, &self.component, &linker).ok()?;

        f(&mut store, &instance).ok()
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
    fn name(&self) -> &str {
        "WASM Device"
    }

    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        self.with_instance(|store, instance| {
            let req = build_request_snapshot(ctx);

            let result = instance
                .snakeway_device_policy()
                .call_on_request(store, &req)?;

            apply_request_result(ctx, result)
        })
        .unwrap_or(DeviceResult::Continue)
    }

    fn on_stream_request_body(
        &self,
        ctx: &mut RequestCtx,
        maybe_chunk: &mut Option<Bytes>,
        end_of_stream: bool,
    ) -> DeviceResult {
        let chunk = maybe_chunk.take().map(|bytes| BodyChunk {
            data: bytes.to_vec(),
            end_of_stream,
        });

        self.with_instance(|store, instance| {
            let req = build_request_snapshot(ctx);

            let result = instance
                .snakeway_device_policy()
                .call_on_stream_request_body(store, &req, chunk.as_ref())?;

            if matches!(result.decision, Decision::Block) {
                let request_id = ctx.extensions.get::<RequestId>().map(|id| id.0.clone());
                return Ok(DeviceResult::Respond(block_403(request_id)));
            }

            Ok(DeviceResult::Continue)
        })
        .unwrap_or(DeviceResult::Continue)
    }

    fn before_proxy(&self, ctx: &mut RequestCtx) -> DeviceResult {
        self.with_instance(|store, instance| {
            let req = build_request_snapshot(ctx);

            let result = instance
                .snakeway_device_policy()
                .call_before_proxy(store, &req)?;

            apply_request_result(ctx, result)
        })
        .unwrap_or_else(|| {
            tracing::debug!("WASM device does not implement before_proxy");
            DeviceResult::Continue
        })
    }

    fn after_proxy(&self, _ctx: &mut ResponseCtx) -> DeviceResult {
        tracing::debug!("WASM device does not implement after_proxy");
        DeviceResult::Continue
    }

    fn on_response(&self, _ctx: &mut ResponseCtx) -> DeviceResult {
        tracing::debug!("WASM device does not implement on_response");
        DeviceResult::Continue
    }
}

/// Build a deterministic request snapshot for WASM policy evaluation.
fn build_request_snapshot(ctx: &RequestCtx) -> Request {
    Request {
        original_path: ctx.original_uri_path().to_string(),
        route_path: ctx.canonical_path().to_string(),
        headers: ctx
            .headers()
            .iter()
            .map(|(k, v)| Header {
                name: k.to_string(),
                value: v.to_str().unwrap_or("").to_string(),
            })
            .collect(),
    }
}

/// Apply a request_result returned from WASM to the RequestCtx.
fn apply_request_result(
    ctx: &mut RequestCtx,
    result: crate::execution::device::wasm::bindings::exports::snakeway::device::policy::RequestResult,
) -> Result<DeviceResult> {
    if matches!(result.decision, Decision::Block) {
        let request_id = ctx.extensions.get::<RequestId>().map(|id| id.0.clone());
        return Ok(DeviceResult::Respond(block_403(request_id)));
    }

    if let Some(RequestPatch {
        set_route_path,
        set_upstream_path,
        set_headers,
        remove_headers,
    }) = result.patch
    {
        if let Some(path) = set_route_path {
            ctx.set_canonical_path(path);
        }

        if let Some(path) = set_upstream_path {
            ctx.upstream_path = Some(path);
        }

        for header in set_headers {
            if let (Ok(name), Ok(value)) = (header.name.parse::<HeaderName>(), header.value.parse())
            {
                ctx.insert_header(name, value);
            }
        }

        for name in remove_headers {
            ctx.remove_header(name.as_str());
        }
    }

    Ok(DeviceResult::Continue)
}

/// Standard 403 response for blocked requests.
fn block_403(request_id: Option<String>) -> ResponseCtx {
    ResponseCtx::new(
        request_id,
        StatusCode::FORBIDDEN,
        HeaderMap::new(),
        b"Blocked by device".to_vec(),
    )
}
