use anyhow::Result;
use bytes::Bytes;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use wasmtime::{
    Config, Engine, Store, StoreLimits, StoreLimitsBuilder,
    component::{Component, Linker},
};

const MAX_MEMORY_SIZE: usize = 10 * 1024 * 1024;
const MAX_TABLE_ELEMENTS: usize = 10_000;

use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};

use crate::execution::ctx::{RequestCtx, RequestId, ResponseCtx};
use crate::execution::device::core::{Device, DeviceResult};

use crate::execution::device::wasm::bindings::{
    Device as DeviceBindings, DevicePre,
    exports::snakeway::device::policy,
    snakeway::device::{
        host, types as wit_types,
        types::{
            Action, BodyAction, BodyChunk, Header, HeaderOp, Request, RequestPatch, Response,
            ResponsePatch,
        },
    },
};
use snakeway_conf::types::WasmDeviceFailPolicy;

#[allow(dead_code)]
pub(crate) struct HostState {
    pub(crate) config: Arc<HashMap<String, String>>,
    pub(crate) device_name: Arc<str>,
    pub(crate) limits: StoreLimits,
}

impl wit_types::Host for HostState {}

impl host::Host for HostState {
    fn config_get(&mut self, key: String) -> Option<String> {
        self.config.get(&key).cloned()
    }

    fn log(&mut self, level: u8, message: String) {
        match level {
            0 => tracing::trace!(device = %self.device_name, "{message}"),
            1 => tracing::debug!(device = %self.device_name, "{message}"),
            2 => tracing::info!(device = %self.device_name, "{message}"),
            3 => tracing::warn!(device = %self.device_name, "{message}"),
            _ => tracing::error!(device = %self.device_name, "{message}"),
        }
    }

    fn metric_increment(&mut self, _name: String, _delta: u64) {}

    fn epoch_secs(&mut self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

pub(crate) struct WasmDevice {
    instance_pre: DevicePre<HostState>,
    config: Arc<HashMap<String, String>>,
    device_name: Arc<str>,
    fail_policy: WasmDeviceFailPolicy,
    timeout_epochs: u64,
    #[allow(dead_code)]
    body_buffer_max: u64,
    engine: Arc<Engine>,
}

impl WasmDevice {
    pub(crate) fn load(
        engine: Arc<Engine>,
        path: &PathBuf,
        device_name: String,
        fail_policy: WasmDeviceFailPolicy,
        timeout_ms: u64,
        body_buffer_max: u64,
        config: HashMap<String, String>,
    ) -> Result<Self> {
        let component = Component::from_file(&engine, path)?;

        let mut linker: Linker<HostState> = Linker::new(&engine);
        DeviceBindings::add_to_linker::<_, wasmtime::component::HasSelf<HostState>>(
            &mut linker,
            |state| state,
        )?;

        let raw_pre = linker.instantiate_pre(&component)?;
        let instance_pre = DevicePre::new(raw_pre)?;

        Ok(Self {
            instance_pre,
            config: Arc::new(config),
            device_name: Arc::from(device_name),
            fail_policy,
            timeout_epochs: timeout_ms,
            body_buffer_max,
            engine,
        })
    }

    fn with_instance<F>(&self, hook_name: &str, f: F) -> DeviceResult
    where
        F: FnOnce(&mut Store<HostState>, &DeviceBindings) -> Result<DeviceResult>,
    {
        let limits = StoreLimitsBuilder::new()
            .memory_size(MAX_MEMORY_SIZE)
            .table_elements(MAX_TABLE_ELEMENTS)
            .build();

        let host_state = HostState {
            config: Arc::clone(&self.config),
            device_name: Arc::clone(&self.device_name),
            limits,
        };

        let mut store = Store::new(&self.engine, host_state);
        store.limiter(|state| &mut state.limits);
        store.set_epoch_deadline(self.timeout_epochs);
        store.epoch_deadline_trap();

        let instance = match self.instance_pre.instantiate(&mut store) {
            Ok(inst) => inst,
            Err(e) => {
                return self.handle_failure(hook_name, "instantiation", &anyhow::anyhow!("{e}"));
            }
        };

        match f(&mut store, &instance) {
            Ok(result) => result,
            Err(e) => self.handle_failure(hook_name, "execution", &e),
        }
    }

    fn handle_failure(&self, hook: &str, phase: &str, error: &anyhow::Error) -> DeviceResult {
        match self.fail_policy {
            WasmDeviceFailPolicy::Open => {
                tracing::warn!(
                    device = %self.device_name,
                    hook,
                    phase,
                    error = %error,
                    "wasm device failure (fail-open: continuing)"
                );
                DeviceResult::Continue
            }
            WasmDeviceFailPolicy::Closed => {
                tracing::error!(
                    device = %self.device_name,
                    hook,
                    phase,
                    error = %error,
                    "wasm device failure (fail-closed: blocking)"
                );
                DeviceResult::Respond(block_503(None))
            }
        }
    }
}

pub(crate) fn create_wasm_engine() -> Result<Arc<Engine>> {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.epoch_interruption(true);

    let engine = Engine::new(&config)?;
    let engine = Arc::new(engine);

    let ticker_engine = Arc::clone(&engine);
    std::thread::Builder::new()
        .name("wasm-epoch-ticker".to_string())
        .spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(1));
                ticker_engine.increment_epoch();
            }
        })?;

    Ok(engine)
}

impl Device for WasmDevice {
    fn name(&self) -> &str {
        &self.device_name
    }

    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        self.with_instance("on_request", |store, instance| {
            let req = build_request_snapshot(ctx);
            let result = instance
                .snakeway_device_policy()
                .call_on_request(store, &req)?;
            apply_request_result(ctx, result)
        })
    }

    fn on_stream_request_body(
        &self,
        ctx: &mut RequestCtx,
        maybe_chunk: &mut Option<Bytes>,
        end_of_stream: bool,
    ) -> DeviceResult {
        let chunk = maybe_chunk.as_ref().map(|bytes| BodyChunk {
            data: bytes.to_vec(),
            end_of_stream,
        });

        self.with_instance("on_stream_request_body", |store, instance| {
            let req = build_request_snapshot(ctx);
            let result = instance
                .snakeway_device_policy()
                .call_on_stream_request_body(store, &req, chunk.as_ref())?;

            let request_id = ctx.extensions.get::<RequestId>().map(|id| id.0.clone());
            apply_body_result(request_id, maybe_chunk, result)
        })
    }

    fn before_proxy(&self, ctx: &mut RequestCtx) -> DeviceResult {
        self.with_instance("before_proxy", |store, instance| {
            let req = build_request_snapshot(ctx);
            let result = instance
                .snakeway_device_policy()
                .call_before_proxy(store, &req)?;
            apply_request_result(ctx, result)
        })
    }

    fn after_proxy(&self, ctx: &mut ResponseCtx) -> DeviceResult {
        self.with_instance("after_proxy", |store, instance| {
            let resp = build_response_snapshot(ctx);
            let result = instance
                .snakeway_device_policy()
                .call_after_proxy(store, &resp)?;
            apply_response_result(ctx, result)
        })
    }

    fn on_stream_response_body(
        &self,
        ctx: &mut ResponseCtx,
        maybe_chunk: &mut Option<Bytes>,
        end_of_stream: bool,
    ) -> DeviceResult {
        let chunk = maybe_chunk.as_ref().map(|bytes| BodyChunk {
            data: bytes.to_vec(),
            end_of_stream,
        });

        self.with_instance("on_stream_response_body", |store, instance| {
            let resp = build_response_snapshot(ctx);
            let result = instance
                .snakeway_device_policy()
                .call_on_stream_response_body(store, &resp, chunk.as_ref())?;

            let request_id = ctx.request_id.clone();
            apply_body_result(request_id, maybe_chunk, result)
        })
    }

    fn on_response(&self, ctx: &mut ResponseCtx) -> DeviceResult {
        self.with_instance("on_response", |store, instance| {
            let resp = build_response_snapshot(ctx);
            let result = instance
                .snakeway_device_policy()
                .call_on_response(store, &resp)?;
            apply_response_result(ctx, result)
        })
    }
}

fn build_request_snapshot(ctx: &RequestCtx) -> Request {
    Request {
        method: ctx.method_str().to_string(),
        scheme: ctx.scheme().to_string(),
        authority: ctx.effective_host().to_string(),
        original_path: ctx.original_uri_path().to_string(),
        route_path: ctx.canonical_path().to_string(),
        query: ctx.query_string().to_string(),
        client_ip: ctx.peer_ip.to_string(),
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

fn build_response_snapshot(ctx: &ResponseCtx) -> Response {
    Response {
        status: ctx.status.as_u16(),
        headers: ctx
            .headers
            .iter()
            .map(|(k, v)| Header {
                name: k.to_string(),
                value: v.to_str().unwrap_or("").to_string(),
            })
            .collect(),
    }
}

fn handle_action(action: Action, request_id: Option<String>) -> Result<DeviceResult> {
    match action {
        Action::Continue => Ok(DeviceResult::Continue),
        Action::Block => Ok(DeviceResult::Respond(block_403(request_id))),
        Action::Respond(synthetic) => {
            let status =
                StatusCode::from_u16(synthetic.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let mut headers = HeaderMap::new();
            for h in synthetic.headers {
                if let (Ok(name), Ok(value)) = (
                    h.name.parse::<HeaderName>(),
                    HeaderValue::from_str(&h.value),
                ) {
                    headers.insert(name, value);
                }
            }
            Ok(DeviceResult::Respond(ResponseCtx::new(
                request_id,
                status,
                headers,
                synthetic.body,
            )))
        }
    }
}

fn apply_request_header_ops(ctx: &mut RequestCtx, ops: Vec<HeaderOp>) {
    for op in ops {
        match op {
            HeaderOp::Set(h) => {
                if let (Ok(name), Ok(value)) = (
                    h.name.parse::<HeaderName>(),
                    HeaderValue::from_str(&h.value),
                ) {
                    ctx.insert_header(name, value);
                }
            }
            HeaderOp::Append(h) => {
                if let (Ok(name), Ok(value)) = (
                    h.name.parse::<HeaderName>(),
                    HeaderValue::from_str(&h.value),
                ) {
                    ctx.append_header(name, value);
                }
            }
            HeaderOp::Remove(name) => {
                ctx.remove_header(&name);
            }
        }
    }
}

fn apply_header_ops(headers: &mut HeaderMap, ops: Vec<HeaderOp>) {
    for op in ops {
        match op {
            HeaderOp::Set(h) => {
                if let (Ok(name), Ok(value)) = (
                    h.name.parse::<HeaderName>(),
                    HeaderValue::from_str(&h.value),
                ) {
                    headers.insert(name, value);
                }
            }
            HeaderOp::Append(h) => {
                if let (Ok(name), Ok(value)) = (
                    h.name.parse::<HeaderName>(),
                    HeaderValue::from_str(&h.value),
                ) {
                    headers.append(name, value);
                }
            }
            HeaderOp::Remove(name) => {
                if let Ok(name) = name.parse::<HeaderName>() {
                    headers.remove(name);
                }
            }
        }
    }
}

fn apply_request_result(
    ctx: &mut RequestCtx,
    result: policy::RequestResult,
) -> Result<DeviceResult> {
    let request_id = ctx.extensions.get::<RequestId>().map(|id| id.0.clone());

    if !matches!(result.action, Action::Continue) {
        return handle_action(result.action, request_id);
    }

    if let Some(RequestPatch {
        set_route_path,
        set_upstream_path,
        ops,
    }) = result.patch
    {
        if let Some(path) = set_route_path {
            ctx.set_canonical_path(path);
        }
        if let Some(path) = set_upstream_path {
            ctx.upstream_path = Some(path);
        }
        apply_request_header_ops(ctx, ops);
    }

    Ok(DeviceResult::Continue)
}

fn apply_response_result(
    ctx: &mut ResponseCtx,
    result: policy::ResponseResult,
) -> Result<DeviceResult> {
    let request_id = ctx.request_id.clone();

    if !matches!(result.action, Action::Continue) {
        return handle_action(result.action, request_id);
    }

    if let Some(ResponsePatch { set_status, ops }) = result.patch {
        if let Some(status_code) = set_status.and_then(|s| StatusCode::from_u16(s).ok()) {
            ctx.status = status_code;
        }
        apply_header_ops(&mut ctx.headers, ops);
    }

    Ok(DeviceResult::Continue)
}

fn apply_body_result(
    request_id: Option<String>,
    maybe_chunk: &mut Option<Bytes>,
    result: policy::BodyResult,
) -> Result<DeviceResult> {
    match result.action {
        BodyAction::Passthrough => Ok(DeviceResult::Continue),
        BodyAction::Replace(data) => {
            *maybe_chunk = Some(Bytes::from(data));
            Ok(DeviceResult::Continue)
        }
        BodyAction::Drop => {
            *maybe_chunk = None;
            Ok(DeviceResult::Continue)
        }
        BodyAction::Block => Ok(DeviceResult::Respond(block_403(request_id))),
    }
}

fn block_403(request_id: Option<String>) -> ResponseCtx {
    ResponseCtx::new(
        request_id,
        StatusCode::FORBIDDEN,
        HeaderMap::new(),
        b"Blocked by device".to_vec(),
    )
}

fn block_503(request_id: Option<String>) -> ResponseCtx {
    ResponseCtx::new(
        request_id,
        StatusCode::SERVICE_UNAVAILABLE,
        HeaderMap::new(),
        b"Service unavailable".to_vec(),
    )
}
