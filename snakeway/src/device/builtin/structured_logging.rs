use crate::ctx::{RequestCtx, ResponseCtx};
use crate::device::core::{result::DeviceResult, Device};

pub struct StructuredLoggingDevice;

impl StructuredLoggingDevice {
    pub fn new() -> Self {
        StructuredLoggingDevice
    }
}

impl Device for StructuredLoggingDevice {
    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        log::info!(
            "snakeway.device.request: method={} uri={} headers={:?}",
            ctx.method,
            ctx.uri,
            ctx.headers
        );
        DeviceResult::Continue
    }

    fn before_proxy(&self, ctx: &mut RequestCtx) -> DeviceResult {
        log::info!(
            "snakeway.device.before_proxy: method={} uri={} headers={:?}",
            ctx.method,
            ctx.uri,
            ctx.headers
        );
        DeviceResult::Continue
    }

    fn after_proxy(&self, ctx: &mut ResponseCtx) -> DeviceResult {
        log::info!(
            "snakeway.device.after_proxy: status={} headers={:?}",
            ctx.status,
            ctx.headers
        );
        DeviceResult::Continue
    }

    fn on_response(&self, ctx: &mut ResponseCtx) -> DeviceResult {
        log::info!(
            "snakeway.device.response: status={} headers={:?}",
            ctx.status,
            ctx.headers
        );
        DeviceResult::Continue
    }
}
