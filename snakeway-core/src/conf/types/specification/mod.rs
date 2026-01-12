mod admin;
mod bind;
mod device;
pub mod entrypoint;
mod origin;
mod redirect;
mod server;
mod service;
mod static_files;

pub use admin::BindAdminSpec;
pub use bind::BindSpec;
pub use device::{
    DeviceSpec, IdentityDeviceSpec, StructuredLoggingDeviceSpec, UaEngineSpec, WasmDeviceSpec,
};
pub use entrypoint::EntrypointSpec;
pub use origin::Origin;
pub use redirect::RedirectSpec;
use serde::{Deserialize, Serialize};
pub use server::ServerSpec;
pub use service::{LoadBalancingStrategySpec, ServiceRouteSpec, ServiceSpec, UpstreamSpec};
pub use static_files::{CachePolicySpec, CompressionOptsSpec, StaticFilesSpec, StaticRouteSpec};

/// The operator DSL for the config subsystem.
/// This defines the configuration file format of files in ./config/ingress.d/*.hcl
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct IngressSpec {
    pub bind: Option<BindSpec>,
    pub bind_admin: Option<BindAdminSpec>,
    pub redirect_cfgs: Vec<RedirectSpec>,
    pub service_cfgs: Vec<ServiceSpec>,
    pub static_cfgs: Vec<StaticFilesSpec>,
}
