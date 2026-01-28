use super::{Device, DeviceResult};
use crate::ctx::{RequestCtx, ResponseCtx, WsCloseCtx, WsCtx};
use bytes::Bytes;
use std::sync::Arc;

pub struct DevicePipeline;

fn run_device_chain<D>(
    devices: &[D],
    mut f: impl FnMut(&dyn Device) -> DeviceResult,
) -> DeviceResult
where
    D: AsRef<dyn Device>,
{
    for dev in devices {
        let dev_ref = dev.as_ref();
        match f(dev_ref) {
            DeviceResult::Continue => continue,
            r @ DeviceResult::Respond(_) => return r,
            DeviceResult::Error(err) => {
                dev_ref.on_error(&err);
                return DeviceResult::Error(err);
            }
        }
    }
    DeviceResult::Continue
}

fn run_device_chain_no_error<D>(
    devices: &[D],
    mut f: impl FnMut(&dyn Device) -> DeviceResult,
) -> DeviceResult
where
    D: AsRef<dyn Device>,
{
    for dev in devices {
        match f(dev.as_ref()) {
            DeviceResult::Continue => continue,
            r => return r,
        }
    }
    DeviceResult::Continue
}

/// Device pipeline for WebSocket events
impl DevicePipeline {
    pub(crate) fn run_on_ws_open(devices: &[Arc<dyn Device>], ctx: &WsCtx) {
        for dev in devices {
            dev.on_ws_open(ctx);
        }
    }

    pub(crate) fn run_on_ws_close(devices: &[Arc<dyn Device>], ctx: &WsCloseCtx) {
        for dev in devices {
            dev.on_ws_close(ctx);
        }
    }
}

/// Device pipeline for HTTP events
impl DevicePipeline {
    pub fn run_on_request(devices: &[Arc<dyn Device>], ctx: &mut RequestCtx) -> DeviceResult {
        run_device_chain_no_error(devices, |dev| dev.on_request(ctx))
    }

    pub fn on_stream_request_body(
        devices: &[Arc<dyn Device>],
        ctx: &mut RequestCtx,
        body: &mut Option<Bytes>,
        end_of_stream: bool,
    ) -> DeviceResult {
        run_device_chain(devices, |dev| {
            dev.on_stream_request_body(ctx, body, end_of_stream)
        })
    }

    pub fn run_before_proxy(
        devices: &[impl AsRef<dyn Device>],
        ctx: &mut RequestCtx,
    ) -> DeviceResult {
        run_device_chain(devices, |dev| dev.before_proxy(ctx))
    }

    pub fn run_after_proxy(
        devices: &[impl AsRef<dyn Device>],
        ctx: &mut ResponseCtx,
    ) -> DeviceResult {
        run_device_chain(devices, |dev| dev.after_proxy(ctx))
    }

    pub fn run_on_response(
        devices: &[impl AsRef<dyn Device>],
        ctx: &mut ResponseCtx,
    ) -> DeviceResult {
        run_device_chain(devices, |dev| dev.on_response(ctx))
    }
}
