use crate::conf::types::shared::TlsConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListenerConfig {
    /// Name of the listener. Must be unique among listeners.
    pub name: String,

    /// Address to bind, e.g. "0.0.0.0:8080"
    pub addr: String,

    /// Optional TLS config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsConfig>,

    /// Enable HTTP/2 on this listener.
    #[serde(default)]
    pub enable_http2: bool,

    /// Whether a listener serves admin endpoints or not.
    #[serde(default)]
    pub enable_admin: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect: Option<RedirectConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedirectConfig {
    pub to: String,
    pub status: u16,
}
