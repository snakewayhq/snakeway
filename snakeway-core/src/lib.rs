extern crate core;

//-----------------------------------------------------------------------------
// Private
//-----------------------------------------------------------------------------
mod control_plane;
mod data_plane;
mod execution;
mod net;
mod serialization;

//-----------------------------------------------------------------------------
// Public Subsystems
//-----------------------------------------------------------------------------
pub mod cli;
pub mod conf;
pub mod http_event;

//-----------------------------------------------------------------------------
// Entry points
//-----------------------------------------------------------------------------

pub use crate::execution::device;
pub use control_plane::bootstrap::start_server;
pub use data_plane::bootstrap::build_pingora_server;
