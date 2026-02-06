mod connection_filter;

use crate::conf::types::shared::TlsConfig;
use crate::conf::types::{BindAdminSpec, BindSpec};
pub use connection_filter::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListenerConfig {
    /// Name of the listener. Must be unique among listeners.
    pub name: String,

    /// Address to bind, e.g. "0.0.0.0:8080"
    pub addr: String,

    /// Optional TLS config.
    pub tls: Option<TlsConfig>,

    /// Enable HTTP/2 on this listener.
    pub enable_http2: bool,

    /// Whether a listener serves admin endpoints or not.
    pub enable_admin: bool,

    /// Optional redirect config.
    pub redirect: Option<RedirectConfig>,

    pub connection_filter: Option<ConnectionFilterConfig>,
}

impl ListenerConfig {
    pub fn from_redirect(
        name: &str,
        from_addr: String,
        redirect_response_code: u16,
        spec: BindSpec,
    ) -> Self {
        let addr = spec.resolve().expect("failed to resolve bind address");
        Self {
            name: name.to_string(),
            addr: from_addr,
            tls: None,
            enable_http2: false,
            enable_admin: false,
            redirect: Some(RedirectConfig::new(
                addr.to_string(),
                redirect_response_code,
            )),
            connection_filter: spec.connection_filter.map(Into::into),
        }
    }

    pub fn from_bind(name: &str, spec: BindSpec) -> Self {
        Self {
            name: name.to_string(),
            addr: spec
                .resolve()
                .expect("failed to resolve bind address")
                .to_string(),
            tls: spec.tls.map(Into::into),
            enable_http2: spec.enable_http2,
            enable_admin: false,
            redirect: None,
            connection_filter: spec.connection_filter.map(Into::into),
        }
    }

    pub fn from_bind_admin(name: &str, spec: BindAdminSpec) -> Self {
        Self {
            name: name.to_string(),
            addr: spec
                .resolve()
                .expect("failed to resolve bind address")
                .to_string(),
            tls: Some(spec.tls.into()),
            enable_http2: false,
            enable_admin: true,
            redirect: None,
            connection_filter: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedirectConfig {
    pub destination: String,
    pub response_code: u16,
}

impl RedirectConfig {
    pub fn new(destination: String, response_code: u16) -> Self {
        Self {
            destination,
            response_code,
        }
    }
}
