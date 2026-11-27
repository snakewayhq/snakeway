//! Passthrough helpers the fault devices share for the hooks they do not
//! fault on.

use crate::snakeway::device::types::{
    Action, BodyAction, BodyResult, RequestResult, ResponseResult,
};

pub fn continue_request() -> RequestResult {
    RequestResult {
        action: Action::Continue,
        patch: None,
    }
}

pub fn continue_response() -> ResponseResult {
    ResponseResult {
        action: Action::Continue,
        patch: None,
    }
}

pub fn passthrough_body() -> BodyResult {
    BodyResult {
        action: BodyAction::Passthrough,
    }
}
