pub mod handler;
mod resolve;
mod response;
mod serve;

pub use handler::handle_static_request;
pub use response::{ConditionalHeaders, ServeError, StaticBody, StaticResponse};
