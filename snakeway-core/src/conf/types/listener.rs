use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListenerConfig {
    /// Address to bind, e.g. "0.0.0.0:8080"
    pub addr: String,

    /// Optional TLS config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsConfig>,
}

/// Paths are validated and resolved during config validation.
/// Runtime code assumes these values are valid.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TlsConfig {
    pub cert: String,
    pub key: String,
}
