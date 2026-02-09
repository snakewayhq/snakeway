use crate::ctx::{RequestCtx, ResponseCtx, WsCloseCtx, WsCtx};
use crate::device::core::errors::DeviceError;
use crate::device::core::{Device, DeviceResult};
use bytes::Bytes;

pub struct RequestRateLimitingDevice;

impl Device for RequestRateLimitingDevice {
    fn name(&self) -> &str {
        "Request Rate Limit"
    }

    fn on_request(&self, _ctx: &mut RequestCtx) -> DeviceResult {
        todo!()
    }

    fn on_stream_request_body(
        &self,
        _ctx: &mut RequestCtx,
        _maybe_chunk: &mut Option<Bytes>,
        _end_of_stream: bool,
    ) -> DeviceResult {
        todo!()
    }

    fn before_proxy(&self, _ctx: &mut RequestCtx) -> DeviceResult {
        todo!()
    }

    fn after_proxy(&self, _ctx: &mut ResponseCtx) -> DeviceResult {
        todo!()
    }

    fn on_response(&self, _ctx: &mut ResponseCtx) -> DeviceResult {
        todo!()
    }

    fn on_ws_open(&self, _ctx: &WsCtx) {
        todo!()
    }

    fn on_ws_close(&self, _ctx: &WsCloseCtx) {
        todo!()
    }

    fn on_error(&self, _err: &DeviceError) {
        todo!()
    }
}
