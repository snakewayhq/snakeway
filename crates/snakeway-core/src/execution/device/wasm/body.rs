use crate::execution::device::core::DeviceResult;
use crate::execution::device::wasm::bindings::exports::snakeway::device::policy;
use crate::execution::device::wasm::bindings::snakeway::device::types::BodyAction;
use crate::execution::device::wasm::response::block_403;
use bytes::Bytes;

pub(crate) fn apply_body_result(
    request_id: Option<String>,
    maybe_chunk: &mut Option<Bytes>,
    result: policy::BodyResult,
) -> anyhow::Result<DeviceResult> {
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
