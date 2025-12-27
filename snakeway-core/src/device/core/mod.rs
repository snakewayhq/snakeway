pub mod errors;
pub mod pipeline;
pub mod registry;
pub mod result;

use self::errors::DeviceError;
pub(crate) use self::result::DeviceResult;
use crate::ctx::{RequestCtx, ResponseCtx, WsCloseCtx, WsCtx};

/// A trait representing a processing unit in the HTTP proxy pipeline.
///
/// Devices can intercept and modify requests/responses at different stages
/// of the proxy pipeline. Each device must be both Send and Sync to ensure
/// thread-safety in the async runtime.
///
/// All methods provide default implementations that simply continue the pipeline,
/// allowing implementations to override only the methods they care about.
pub trait Device: Send + Sync {
    /// Called when a request is first received, before any processing.
    ///
    /// This is the first opportunity to inspect or modify the incoming request.
    fn on_request(&self, _ctx: &mut RequestCtx) -> DeviceResult {
        DeviceResult::Continue
    }

    /// Called immediately before the request is proxied to the upstream server.
    ///
    /// Last chance to modify the request before it's sent upstream.
    fn before_proxy(&self, _ctx: &mut RequestCtx) -> DeviceResult {
        DeviceResult::Continue
    }

    /// Called after receiving the response from upstream, but before processing.
    ///
    /// First opportunity to inspect or modify the upstream response.
    fn after_proxy(&self, _ctx: &mut ResponseCtx) -> DeviceResult {
        DeviceResult::Continue
    }

    /// Called just before sending the response back to the client.
    ///
    /// Final opportunity to modify the response before it's sent to the client.
    fn on_response(&self, _ctx: &mut ResponseCtx) -> DeviceResult {
        DeviceResult::Continue
    }

    /// Called when a WebSocket connection is opened.
    fn on_ws_open(&self, _ctx: &WsCtx) {}

    /// Called when a WebSocket connection is closed.
    fn on_ws_close(&self, _ctx: &WsCloseCtx) {}

    /// Called when an error occurs during request processing.
    ///
    /// Provides an opportunity to handle or log errors in the pipeline.
    fn on_error(&self, _err: &DeviceError) {}
}
