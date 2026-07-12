extern crate core;

//-----------------------------------------------------------------------------
// Private / Internal
//-----------------------------------------------------------------------------

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

pub use cli::bootstrap::run;
pub use server::start_server;

#[doc(hidden)]
pub mod testing_api {
    pub use crate::cli;
    pub use crate::control_plane::{ControlPlaneServer, Metrics, RuntimeServer};
    pub use snakeway_conf as conf;
}

#[doc(hidden)]
pub mod bench_api {
    pub use crate::execution::ctx::request::normalization::{
        ProtocolNormalizationMode, normalize_headers,
    };
    pub use crate::execution::ctx::request::request_ctx::RequestCtx;
    pub use crate::execution::device::builtin::identity::IdentityDevice;
    pub use crate::execution::device::builtin::request_filter::RequestFilterDevice;
    pub use crate::execution::device::core::{Device, DevicePipeline};
    pub use crate::execution::route::types::RouteId;
    pub use crate::execution::route::{RouteRuntime, Router};
    pub use snakeway_conf::types::{IdentityDeviceConfig, RequestFilterDeviceConfig, UaEngineKind};
}
