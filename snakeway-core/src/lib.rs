extern crate core;

//-----------------------------------------------------------------------------
// Private / Internal
//-----------------------------------------------------------------------------

mod cli;
mod conf;
mod control_plane;
mod data_plane;
mod execution;
mod http_event;
mod net;
mod serialization;
mod server;

//-----------------------------------------------------------------------------
// Public API
//-----------------------------------------------------------------------------

//-----------------------------------------------------------------------------
// Entry points
//-----------------------------------------------------------------------------
pub use cli::bootstrap::run_cli;
pub use data_plane::bootstrap::build_pingora_server;
pub use server::start_server;
