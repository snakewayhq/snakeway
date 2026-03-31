mod connection_rate_limiting_filter_config;
mod connection_rate_limiting_filter_lower;
mod listener_lower;
mod network_connection_filter_config;
mod network_connection_filter_lower;
mod tls_termination_config;
mod tls_termination_lower;

pub use connection_rate_limiting_filter_config::*;
pub use network_connection_filter_config::*;
pub use tls_termination_config::*;

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

    /// Optional redirect config.
    pub redirect: Option<RedirectConfig>,

    pub connection_filter: Option<NetworkConnectionFilterConfig>,

    pub connection_rate_limiting_filter: Option<ConnectionRateLimitingFilterConfig>,
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
