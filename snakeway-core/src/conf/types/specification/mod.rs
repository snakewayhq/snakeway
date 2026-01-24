mod bind;
mod bind_admin;
mod bind_interface;
mod device;
pub mod entrypoint;
mod origin;
mod server;
mod service;
mod static_files;
mod tls;

pub use bind::{BindSpec, RedirectSpec};
pub use bind_admin::BindAdminSpec;
pub use bind_interface::{BindInterfaceInput, BindInterfaceSpec};
pub use device::{
    DeviceSpec, IdentityDeviceSpec, RequestFilterDeviceSpec, StructuredLoggingDeviceSpec,
    UaEngineSpec, WasmDeviceSpec,
};
pub use entrypoint::EntrypointSpec;
pub use origin::Origin;
use serde::{Deserialize, Serialize};
pub use server::ServerSpec;
pub use service::{
    EndpointSpec, HostSpec, LoadBalancingStrategySpec, ServiceRouteSpec, ServiceSpec, UpstreamSpec,
};
pub use static_files::{CachePolicySpec, CompressionOptsSpec, StaticFilesSpec, StaticRouteSpec};
pub use tls::TlsSpec;

/// The operator DSL for the config subsystem.
/// This defines the configuration file format of files in ./config/ingress.d/*.hcl
#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct IngressSpec {
    #[serde(skip)]
    pub origin: Origin,
    pub bind: Option<BindSpec>,
    pub bind_admin: Option<BindAdminSpec>,
    pub services: Vec<ServiceSpec>,
    pub static_files: Vec<StaticFilesSpec>,
}
