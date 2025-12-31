use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListenerConfig {
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
}

/// Paths are validated and resolved during config validation.
/// Runtime code assumes these values are valid.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TlsConfig {
    pub cert: String,
    pub key: String,
}
