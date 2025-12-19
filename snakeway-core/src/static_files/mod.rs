pub mod handler;
mod resolve;
mod serve;

pub use handler::handle_static_request;
pub use serve::{ConditionalHeaders, ServeError, StaticBody, StaticResponse};
