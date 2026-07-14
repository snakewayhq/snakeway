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

#[doc(hidden)]
pub mod bench_api {
    pub use snakeway_conf::types::{IdentityDeviceConfig, RequestFilterDeviceConfig, UaEngineKind};
    pub use snakeway_engine::ctx::request::normalization::{
        ProtocolNormalizationMode, normalize_headers,
    };
    pub use snakeway_engine::ctx::request::request_ctx::RequestCtx;
    pub use snakeway_engine::device::builtin::identity::IdentityDevice;
    pub use snakeway_engine::device::builtin::request_filter::RequestFilterDevice;
    pub use snakeway_engine::device::core::{Device, DevicePipeline};
    pub use snakeway_engine::route::types::RouteId;
    pub use snakeway_engine::route::{RouteRuntime, Router};
}
