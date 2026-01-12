use crate::conf::types::shared::TlsConfig;
use crate::conf::types::{BindAdminSpec, BindSpec, RedirectSpec};
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
}

impl ListenerConfig {
    pub fn from_redirect(name: String, spec: RedirectSpec) -> Self {
        Self {
            name,
            addr: spec.addr.clone(),
            tls: None,
            enable_http2: false,
            enable_admin: false,
            redirect: Some(spec.into()),
        }
    }

    pub fn from_bind(name: String, spec: BindSpec) -> Self {
        Self {
            name,
            addr: spec.addr,
            tls: spec.tls,
            enable_http2: spec.enable_http2,
            enable_admin: false,
            redirect: None,
        }
    }

    pub fn from_bind_admin(name: String, spec: BindAdminSpec) -> Self {
        Self {
            name,
            addr: spec.addr,
            tls: Some(spec.tls),
            enable_http2: false,
            enable_admin: true,
            redirect: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedirectConfig {
    pub to: String,
    pub status: u16,
}

impl From<RedirectSpec> for RedirectConfig {
    fn from(spec: RedirectSpec) -> Self {
        Self {
            to: spec.to,
            status: spec.status,
        }
    }
}
