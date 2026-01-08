mod expose_admin;
mod expose_redirect;
mod expose_service;
mod expose_static;

pub use expose_admin::ExposeAdminConfig;
pub use expose_redirect::ExposeRedirectConfig;
pub use expose_service::ExposeServiceConfig;
pub use expose_static::ExposeStaticConfig;
use serde::Deserialize;

/// The operator DSL for the config subsystem.
/// This defines the configuration file format of files in ./config/ingress.d/*.toml
#[derive(Debug, Deserialize)]
pub enum ExposeConfig {
    Redirect(ExposeRedirectConfig),
    Service(ExposeServiceConfig),
    Static(ExposeStaticConfig),
    Admin(ExposeAdminConfig),
}
