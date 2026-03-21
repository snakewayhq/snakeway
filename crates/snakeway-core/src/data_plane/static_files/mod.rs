pub(crate) mod handler;
mod render;
mod resolve;
mod response;

pub(crate) use handler::handle_static_request;
pub(crate) use response::{ConditionalHeaders, ServeError, StaticBody, StaticResponse};
