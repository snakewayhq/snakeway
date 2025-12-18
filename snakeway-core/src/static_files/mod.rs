mod resolve;
mod serve;
pub mod handler;


pub use handler::handle_static_request;
pub use serve::{ServeError, StaticBody, StaticResponse};
