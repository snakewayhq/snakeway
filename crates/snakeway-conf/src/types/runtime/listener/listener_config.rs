use crate::types::{
    AdminAuthConfig, ConnectionRateLimitingFilterConfig, NetworkConnectionFilterConfig,
    TlsTerminationConfig,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListenerConfig {
    /// Name of the listener. Must be unique among listeners.
    pub name: String,

    /// Address to bind, e.g. "0.0.0.0:8080"
    pub addr: String,

    /// Optional TLS termination config.
    pub tls_termination: Option<TlsTerminationConfig>,

    /// Enable HTTP/2 on this listener.
    pub enable_http2: bool,

    /// Whether a listener serves admin endpoints or not.
    pub enable_admin: bool,

    /// Admin authentication config. Populated only on listeners where
    /// `enable_admin` is true.
    #[serde(default)]
    pub admin_auth: Option<AdminAuthConfig>,

    /// Optional redirect config.
    pub redirect: Option<RedirectConfig>,

    pub connection_filter: Option<NetworkConnectionFilterConfig>,

    pub connection_rate_limiting_filter: Option<ConnectionRateLimitingFilterConfig>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RedirectConfig {
    pub destination: String,
    pub response_code: u16,
}
