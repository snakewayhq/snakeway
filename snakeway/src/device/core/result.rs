use crate::ctx::ResponseCtx;
use crate::device::core::errors::DeviceError;

#[derive(Debug)]
pub enum DeviceResult {
    /// Continue to the next device / next phase
    Continue,

    /// Stop the pipeline and immediately return this response to the client
    Respond(ResponseCtx),

    /// Error that should invoke on_error handlers
    #[allow(dead_code)]
    Error(DeviceError),
}
