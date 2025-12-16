use std::sync::Arc;
use super::{Device, DeviceResult};
use crate::ctx::{RequestCtx, ResponseCtx};

pub struct DevicePipeline;

impl DevicePipeline {
    pub fn run_on_request(
        devices: &[Arc<dyn Device>],
        ctx: &mut RequestCtx,
    ) -> DeviceResult {
        for dev in devices {
            match dev.on_request(ctx) {
                DeviceResult::Continue => continue,
                r => return r,
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
                r @ DeviceResult::Error(_) => return r,
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
                r @ DeviceResult::Error(_) => return r,
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
                r @ DeviceResult::Error(_) => return r,
            }
        }
        DeviceResult::Continue
    }
}
