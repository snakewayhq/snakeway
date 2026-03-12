mod connection_rate_limiting_filter;
mod network_connection_filter;
mod tls_termination;

pub use connection_rate_limiting_filter::*;
pub use network_connection_filter::*;
pub use tls_termination::*;

use crate::types::{BindAdminSpec, BindSpec};
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

impl ListenerConfig {
    pub fn from_redirect(
        name: &str,
        from_addr: String,
        redirect_response_code: u16,
        spec: BindSpec,
    ) -> Result<Self, String> {
        let addr = spec.resolve().map_err(|err| err.to_string())?;
        let connection_filter = spec.connection_filter.map(TryInto::try_into).transpose()?;
        Ok(Self {
            name: name.to_string(),
            addr: from_addr,
            tls_termination: None,
            enable_http2: false,
            enable_admin: false,
            redirect: Some(RedirectConfig::new(
                addr.to_string(),
                redirect_response_code,
            )),
            connection_filter,
            connection_rate_limiting_filter: spec.connection_rate_limiting_filter.map(Into::into),
        })
    }

    pub fn from_bind(name: &str, spec: BindSpec) -> Result<Self, String> {
        let addr = spec.resolve().map_err(|err| err.to_string())?;
        let maybe_tls = if let Some(tls) = spec.tls {
            Some(TlsTerminationConfig::try_from(tls).map_err(|err| err.to_string())?)
        } else {
            None
        };
        let connection_filter = spec.connection_filter.map(TryInto::try_into).transpose()?;
        Ok(Self {
            name: name.to_string(),
            addr: addr.to_string(),
            tls_termination: maybe_tls,
            enable_http2: spec.enable_http2,
            enable_admin: false,
            redirect: None,
            connection_filter,
            connection_rate_limiting_filter: spec.connection_rate_limiting_filter.map(Into::into),
        })
    }

    pub fn from_bind_admin(name: &str, spec: BindAdminSpec) -> Result<Self, String> {
        let addr = spec.resolve().map_err(|err| err.to_string())?;
        let tls = TlsTerminationConfig::try_from(spec.tls).map_err(|err| err.to_string())?;
        Ok(Self {
            name: name.to_string(),
            addr: addr.to_string(),
            tls_termination: Some(tls),
            enable_http2: false,
            enable_admin: true,
            redirect: None,
            connection_filter: None,
            connection_rate_limiting_filter: None,
        })
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
