use super::{Device, DeviceResult};
use crate::ctx::{RequestCtx, ResponseCtx, WsCloseCtx, WsCtx};
use bytes::Bytes;
use std::sync::Arc;

pub struct DevicePipeline;

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
        for dev in devices {
            match dev.on_request(ctx) {
                DeviceResult::Continue => continue,
                r => return r,
            }
        }
        DeviceResult::Continue
    }

    pub fn on_stream_request_body(
        devices: &[Arc<dyn Device>],
        ctx: &mut RequestCtx,
        body: &mut Option<Bytes>,
        end_of_stream: bool,
    ) -> DeviceResult {
        for dev in devices {
            match dev.on_stream_request_body(ctx, body, end_of_stream) {
                DeviceResult::Continue => continue,
                r @ DeviceResult::Respond(_) => return r,
                DeviceResult::Error(err) => {
                    dev.as_ref().on_error(&err);
                    return DeviceResult::Error(err);
                }
            }
        }
        DeviceResult::Continue
    }

    pub fn run_before_proxy(
        devices: &[impl AsRef<dyn Device>],
        ctx: &mut RequestCtx,
    ) -> DeviceResult {
        for dev in devices {
            match dev.as_ref().before_proxy(ctx) {
                DeviceResult::Continue => continue,
                r @ DeviceResult::Respond(_) => return r,
                DeviceResult::Error(err) => {
                    dev.as_ref().on_error(&err);
                    return DeviceResult::Error(err);
                }
            }
        }
        DeviceResult::Continue
    }

    pub fn run_after_proxy(
        devices: &[impl AsRef<dyn Device>],
        ctx: &mut ResponseCtx,
    ) -> DeviceResult {
        for dev in devices {
            match dev.as_ref().after_proxy(ctx) {
                DeviceResult::Continue => continue,
                r @ DeviceResult::Respond(_) => return r,
                DeviceResult::Error(err) => {
                    dev.as_ref().on_error(&err);
                    return DeviceResult::Error(err);
                }
            }
        }
        DeviceResult::Continue
    }

    pub fn run_on_response(
        devices: &[impl AsRef<dyn Device>],
        ctx: &mut ResponseCtx,
    ) -> DeviceResult {
        for dev in devices {
            match dev.as_ref().on_response(ctx) {
                DeviceResult::Continue => continue,
                r @ DeviceResult::Respond(_) => return r,
                DeviceResult::Error(err) => {
                    dev.as_ref().on_error(&err);
                    return DeviceResult::Error(err);
                }
            }
        }
        DeviceResult::Continue
    }
}
