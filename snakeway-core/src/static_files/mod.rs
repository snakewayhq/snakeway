pub mod handler;
mod policy;
mod render;
mod resolve;
mod response;

pub use handler::handle_static_request;
pub use response::{ConditionalHeaders, ServeError, StaticBody, StaticResponse};
