use super::bindings::{Device as DeviceBindings, DevicePre};
use super::lifecycle::apply_body_result;
use super::lifecycle::build_request_snapshot;
use super::lifecycle::{
    apply_request_result, apply_response_result, block_503, build_response_snapshot,
};
use super::state::HostState;
use crate::execution::ctx::{RequestCtx, RequestId, ResponseCtx};
use crate::execution::device::core::{Device, DeviceResult};
use crate::execution::device::wasm::bindings::snakeway::device::types::BodyChunk;
use anyhow::Result;
use bytes::Bytes;
use http::{HeaderMap, StatusCode};
use opentelemetry::{
    KeyValue,
    metrics::{Counter, Histogram},
};
use snakeway_conf::types::{WasmDeviceConfig, WasmDeviceFailPolicy, WasmHookMask};
use std::collections::HashMap;
use std::sync::Arc;
use wasmtime::{
    Engine, Store, StoreLimitsBuilder,
    component::{Component, Linker},
};

const MAX_MEMORY_SIZE: usize = 64 * 1024 * 1024;
const MAX_TABLE_ELEMENTS: usize = 10_000;

#[derive(Default, Clone)]
struct WasmBodyBuffers {
    request: HashMap<Arc<str>, Vec<u8>>,
}

pub(crate) struct WasmDevice {
    instance_pre: DevicePre<HostState>,
    config: Arc<HashMap<String, String>>,
    device_name: Arc<str>,
    fail_policy: WasmDeviceFailPolicy,
    timeout_ms: u64,
    body_buffer_max: u64,
    hooks: WasmHookMask,
    engine: Arc<Engine>,
    hook_duration: Histogram<f64>,
    failures: Counter<u64>,
    custom_metrics: Counter<u64>,
}

impl WasmDevice {
    pub(crate) fn load(engine: Arc<Engine>, cfg: &WasmDeviceConfig) -> Result<Self> {
        let component = Component::from_file(&engine, &cfg.path)?;

        let mut linker: Linker<HostState> = Linker::new(&engine);
        DeviceBindings::add_to_linker::<_, wasmtime::component::HasSelf<HostState>>(
            &mut linker,
            |state| state,
        )?;

        let raw_pre = linker.instantiate_pre(&component)?;
        let instance_pre = DevicePre::new(raw_pre)?;

        let meter = opentelemetry::global::meter("snakeway");
        let hook_duration = meter
            .f64_histogram("snakeway.wasm.device.hook_duration_ms")
            .with_description("WASM device hook execution duration in milliseconds")
            .with_unit("ms")
            .build();
        let failures = meter
            .u64_counter("snakeway.wasm.device.failures")
            .with_description("WASM device failure count")
            .build();
        let custom_metrics = meter
            .u64_counter("snakeway.wasm.device.custom")
            .with_description("Guest-emitted custom metrics")
            .build();

        Ok(Self {
            instance_pre,
            config: Arc::new(cfg.config.clone()),
            device_name: Arc::from(cfg.name.clone()),
            fail_policy: cfg.fail_policy.clone(),
            timeout_ms: cfg.timeout_ms,
            body_buffer_max: cfg.body_buffer_max,
            hooks: cfg.hooks,
            engine,
            hook_duration,
            failures,
            custom_metrics,
        })
    }

    fn with_instance<F>(&self, hook_name: &str, f: F) -> DeviceResult
    where
        F: FnOnce(&mut Store<HostState>, &DeviceBindings) -> Result<DeviceResult>,
    {
        let start = std::time::Instant::now();
        let attrs = [
            KeyValue::new("device", self.device_name.to_string()),
            KeyValue::new("hook", hook_name.to_string()),
        ];

        let mut store = hotpath::measure_block!("wasm.store_setup", {
            let limits = StoreLimitsBuilder::new()
                .memory_size(MAX_MEMORY_SIZE)
                .table_elements(MAX_TABLE_ELEMENTS)
                .build();

            let host_state = HostState {
                config: Arc::clone(&self.config),
                device_name: Arc::clone(&self.device_name),
                limits,
                custom_metrics: self.custom_metrics.clone(),
            };

            let mut store = Store::new(&self.engine, host_state);
            store.limiter(|state| &mut state.limits);
            let tick_ms = super::engine::EPOCH_TICK_MS;
            let deadline_ticks = self.timeout_ms.div_ceil(tick_ms);
            store.set_epoch_deadline(deadline_ticks.max(1));
            store.epoch_deadline_trap();
            store
        });

        let instance = match hotpath::measure_block!(
            "wasm.instantiate",
            self.instance_pre.instantiate(&mut store)
        ) {
            Ok(inst) => inst,
            Err(e) => {
                return self.handle_failure(hook_name, "instantiation", &anyhow::anyhow!("{e}"));
            }
        };

        let result = match f(&mut store, &instance) {
            Ok(result) => result,
            Err(e) => self.handle_failure(hook_name, "execution", &e),
        };

        self.hook_duration
            .record(start.elapsed().as_secs_f64() * 1000.0, &attrs);

        result
    }

    fn handle_buffer_overflow(
        &self,
        buffer: Vec<u8>,
        maybe_chunk: &mut Option<Bytes>,
        request_id: Option<String>,
    ) -> DeviceResult {
        self.failures.add(
            1,
            &[
                KeyValue::new("device", self.device_name.to_string()),
                KeyValue::new("reason", "body_buffer_overflow"),
            ],
        );

        match self.fail_policy {
            WasmDeviceFailPolicy::Open => {
                tracing::warn!(
                    device = %self.device_name,
                    buffer_size = buffer.len(),
                    limit = self.body_buffer_max,
                    "body buffer overflow (fail-open: passing through)"
                );
                *maybe_chunk = Some(Bytes::from(buffer));
                DeviceResult::Continue
            }
            WasmDeviceFailPolicy::Closed => {
                tracing::error!(
                    device = %self.device_name,
                    buffer_size = buffer.len(),
                    limit = self.body_buffer_max,
                    "body buffer overflow (fail-closed: blocking)"
                );
                DeviceResult::Respond(ResponseCtx::new(
                    request_id,
                    StatusCode::PAYLOAD_TOO_LARGE,
                    HeaderMap::new(),
                    b"Payload too large".to_vec(),
                ))
            }
        }
    }

    fn handle_failure(&self, hook: &str, phase: &str, error: &anyhow::Error) -> DeviceResult {
        self.failures.add(
            1,
            &[
                KeyValue::new("device", self.device_name.to_string()),
                KeyValue::new("hook", hook.to_string()),
                KeyValue::new("reason", phase.to_string()),
            ],
        );

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

#[hotpath::measure_all]
impl Device for WasmDevice {
    fn name(&self) -> &str {
        &self.device_name
    }

    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        if !self.hooks.on_request {
            return DeviceResult::Continue;
        }
        self.with_instance("on_request", |store, instance| {
            let req = build_request_snapshot(ctx);
            let result = hotpath::measure_block!(
                "wasm.guest.on_request",
                instance
                    .snakeway_device_policy()
                    .call_on_request(store, &req)
            )?;
            apply_request_result(ctx, result)
        })
    }

    fn on_stream_request_body(
        &self,
        ctx: &mut RequestCtx,
        maybe_chunk: &mut Option<Bytes>,
        end_of_stream: bool,
    ) -> DeviceResult {
        if !self.hooks.on_stream_request_body {
            return DeviceResult::Continue;
        }
        if self.body_buffer_max > 0 {
            let buffers = ctx.extensions.get_or_insert_default::<WasmBodyBuffers>();
            let buffer = buffers
                .request
                .entry(Arc::clone(&self.device_name))
                .or_default();

            if let Some(chunk) = maybe_chunk.as_ref() {
                buffer.extend_from_slice(chunk);
            }

            if buffer.len() as u64 > self.body_buffer_max {
                let overflow_buf = buffers
                    .request
                    .remove(&self.device_name)
                    .unwrap_or_default();
                let request_id = ctx.extensions.get::<RequestId>().map(|id| id.0.clone());
                return self.handle_buffer_overflow(overflow_buf, maybe_chunk, request_id);
            }

            if !end_of_stream {
                *maybe_chunk = None;
                return DeviceResult::Continue;
            }

            let full_body = buffers
                .request
                .remove(&self.device_name)
                .unwrap_or_default();
            let assembled = BodyChunk {
                data: full_body,
                end_of_stream: true,
            };

            return self.with_instance("on_stream_request_body", |store, instance| {
                let req = build_request_snapshot(ctx);
                let result = hotpath::measure_block!(
                    "wasm.guest.on_stream_request_body",
                    instance
                        .snakeway_device_policy()
                        .call_on_stream_request_body(store, &req, Some(&assembled))
                )?;

                let request_id = ctx.extensions.get::<RequestId>().map(|id| id.0.clone());
                apply_body_result(request_id, maybe_chunk, result)
            });
        }

        let chunk = maybe_chunk.as_ref().map(|bytes| BodyChunk {
            data: bytes.to_vec(),
            end_of_stream,
        });

        self.with_instance("on_stream_request_body", |store, instance| {
            let req = build_request_snapshot(ctx);
            let result = hotpath::measure_block!(
                "wasm.guest.on_stream_request_body",
                instance
                    .snakeway_device_policy()
                    .call_on_stream_request_body(store, &req, chunk.as_ref())
            )?;

            let request_id = ctx.extensions.get::<RequestId>().map(|id| id.0.clone());
            apply_body_result(request_id, maybe_chunk, result)
        })
    }

    fn before_proxy(&self, ctx: &mut RequestCtx) -> DeviceResult {
        if !self.hooks.before_proxy {
            return DeviceResult::Continue;
        }
        self.with_instance("before_proxy", |store, instance| {
            let req = build_request_snapshot(ctx);
            let result = hotpath::measure_block!(
                "wasm.guest.before_proxy",
                instance
                    .snakeway_device_policy()
                    .call_before_proxy(store, &req)
            )?;
            apply_request_result(ctx, result)
        })
    }

    fn after_proxy(&self, ctx: &mut ResponseCtx) -> DeviceResult {
        if !self.hooks.after_proxy {
            return DeviceResult::Continue;
        }
        self.with_instance("after_proxy", |store, instance| {
            let resp = build_response_snapshot(ctx);
            let result = hotpath::measure_block!(
                "wasm.guest.after_proxy",
                instance
                    .snakeway_device_policy()
                    .call_after_proxy(store, &resp)
            )?;
            apply_response_result(ctx, result)
        })
    }

    fn on_stream_response_body(
        &self,
        ctx: &mut ResponseCtx,
        maybe_chunk: &mut Option<Bytes>,
        end_of_stream: bool,
    ) -> DeviceResult {
        if !self.hooks.on_stream_response_body {
            return DeviceResult::Continue;
        }
        let chunk = maybe_chunk.as_ref().map(|bytes| BodyChunk {
            data: bytes.to_vec(),
            end_of_stream,
        });

        self.with_instance("on_stream_response_body", |store, instance| {
            let resp = build_response_snapshot(ctx);
            let result = hotpath::measure_block!(
                "wasm.guest.on_stream_response_body",
                instance
                    .snakeway_device_policy()
                    .call_on_stream_response_body(store, &resp, chunk.as_ref())
            )?;

            let request_id = ctx.request_id.clone();
            apply_body_result(request_id, maybe_chunk, result)
        })
    }

    fn on_response(&self, ctx: &mut ResponseCtx) -> DeviceResult {
        if !self.hooks.on_response {
            return DeviceResult::Continue;
        }
        self.with_instance("on_response", |store, instance| {
            let resp = build_response_snapshot(ctx);
            let result = hotpath::measure_block!(
                "wasm.guest.on_response",
                instance
                    .snakeway_device_policy()
                    .call_on_response(store, &resp)
            )?;
            apply_response_result(ctx, result)
        })
    }
}
