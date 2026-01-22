use crate::ctx::{RequestCtx, ResponseCtx};
use crate::device::core::{Device, DeviceResult};

#[derive(Debug)]
pub struct RequestFilter {
    pub allow_method_list: Vec<String>,
    pub deny_method_list: Vec<String>,
    pub allow_header_list: Vec<String>,
    pub deny_header_list: Vec<String>,
    pub required_header_list: Vec<String>,
    pub max_header_bytes: usize,
    pub max_body_bytes: usize,
    pub on_match: MatchAction,
}

impl Device for RequestFilter {
    /// RequestFilter is a request-only gate by design
    /// It should only act on ctx.normalized_request
    fn on_request(&self, _: &mut RequestCtx) -> DeviceResult {
        DeviceResult::Continue
        // Matching order...
        // 1. Size limits
        // 2. Methods gates
        // 3. Header gates
    }

    fn before_proxy(&self, _: &mut RequestCtx) -> DeviceResult {
        // RequestFilter is a request-only gate by design
        DeviceResult::Continue
    }

    fn after_proxy(&self, _: &mut ResponseCtx) -> DeviceResult {
        // RequestFilter is a request-only gate by design
        DeviceResult::Continue
    }

    fn on_response(&self, _: &mut ResponseCtx) -> DeviceResult {
        // RequestFilter is a request-only gate by design
        DeviceResult::Continue
    }

    fn on_error(&self, _: &crate::device::core::errors::DeviceError) {}
}

#[derive(Debug)]
pub enum MatchAction {
    Deny { status: u16, reason: &'static str },
    Allow,
}
