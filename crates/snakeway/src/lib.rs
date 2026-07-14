extern crate core;

mod server;

#[doc(hidden)]
pub mod cli;
#[doc(hidden)]
pub mod control_plane;

pub use cli::bootstrap::run;
pub use server::start_server;

#[doc(hidden)]
pub mod testing_api {
    pub use crate::cli;
    pub use crate::control_plane::{ControlPlaneServer, RuntimeServer};
    pub use snakeway_conf as conf;
    pub use snakeway_engine as engine;
    pub use snakeway_observability as observability;
}
