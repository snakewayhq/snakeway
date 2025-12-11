use crate::ctx::ResponseCtx;

#[derive(Debug)]
pub enum DeviceResult {
    /// Continue to the next device / next phase
    Continue,

    /// Stop the pipeline and immediately return this response to the client
    ShortCircuit(ResponseCtx),

    /// Error that should invoke on_error handlers
    Error(String),
}
