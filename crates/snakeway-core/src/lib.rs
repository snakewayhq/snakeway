extern crate core;

//-----------------------------------------------------------------------------
// Private / Internal
//-----------------------------------------------------------------------------

mod net;
mod server;

//-----------------------------------------------------------------------------
// Entry points
//-----------------------------------------------------------------------------
#[doc(hidden)]
pub mod cli;
#[doc(hidden)]
pub mod control_plane;
#[doc(hidden)]
pub mod data_plane;
#[doc(hidden)]
pub mod execution;
pub mod runtime;

pub use cli::bootstrap::run_cli;
pub use server::start_server;

#[doc(hidden)]
pub mod testing_api {
    pub use crate::{cli, control_plane, data_plane, execution};
    pub use snakeway_conf as conf;
}
