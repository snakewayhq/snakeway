mod bind;
pub mod entrypoint;
mod expose_admin;
mod expose_redirect;
mod expose_service;
mod expose_static;

pub use bind::BindConfig;
pub use entrypoint::EntrypointConfig;
pub use expose_admin::BindAdminConfig;
pub use expose_redirect::ExposeRedirectConfig;
pub use expose_service::ExposeServiceConfig;
pub use expose_static::ExposeStaticConfig;
use serde::{Deserialize, Serialize};

/// The operator DSL for the config subsystem.
/// This defines the configuration file format of files in ./config/ingress.d/*.hcl
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExposeConfig {
    Redirect(ExposeRedirectConfig),
    Service(ExposeServiceConfig),
    Static(ExposeStaticConfig),
    Admin(BindAdminConfig),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct IngressConfig {
    pub bind: Option<BindConfig>,
    pub bind_admin: Option<BindAdminConfig>,
    pub redirect_cfgs: Vec<ExposeRedirectConfig>,
    pub service_cfgs: Vec<ExposeServiceConfig>,
    pub static_cfgs: Vec<ExposeStaticConfig>,
}
