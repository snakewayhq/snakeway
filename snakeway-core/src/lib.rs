extern crate core;

//-----------------------------------------------------------------------------
// Private / Internal
//-----------------------------------------------------------------------------

mod http_event;
mod net;
mod serialization;
mod server;

//-----------------------------------------------------------------------------
// Entry points
//-----------------------------------------------------------------------------
#[doc(hidden)]
pub mod cli;
#[doc(hidden)]
pub mod conf;
#[doc(hidden)]
pub mod control_plane;
#[doc(hidden)]
pub mod data_plane;
#[doc(hidden)]
pub mod execution;

pub use cli::bootstrap::run_cli;
pub use server::start_server;

#[cfg(test)]
#[doc(hidden)]
pub mod bench_api {
    pub use crate::conf::types::RequestFilterDeviceConfig;
    pub use crate::conf::types::{IdentityDeviceConfig, UaEngineKind};
    pub use crate::execution::ctx::RequestCtx;
    pub use crate::execution::ctx::normalization::{ProtocolNormalizationMode, normalize_headers};
    pub use crate::execution::device::builtin::request_filter::RequestFilterDevice;
    pub use crate::execution::device::core::Device;
    pub use crate::execution::device::core::pipeline::DevicePipeline;
    pub use crate::execution::route::types::RouteId;
    pub use crate::execution::route::{RouteRuntime, Router};
}

#[doc(hidden)]
pub mod integration_test_api {
    pub use crate::{cli, conf, control_plane, data_plane, execution};
}
