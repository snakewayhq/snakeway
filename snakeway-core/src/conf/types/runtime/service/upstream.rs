use crate::conf::resolution::ResolveError;
use crate::conf::types::EndpointSpec;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpstreamTcpConfig {
    /// e.g. "http://10.0.0.1:8080"
    pub url: String,

    pub weight: u32,

    pub tls: Option<UpstreamTlsConfig>,
}

impl UpstreamTcpConfig {
    pub fn new(weight: u32, spec: &EndpointSpec) -> Result<Self, ResolveError> {
        let protocol = "http";
        let addr = spec.resolve()?;
        Ok(Self {
            weight,
            tls: None,
            url: format!("{protocol}://{addr}"),
        })
    }
}

/// Represent TLS settings for origin server connections.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpstreamTlsConfig {
    pub sni: Option<String>,
    pub verify: bool,
    pub ca_cert: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpstreamUnixConfig {
    /// e.g. "/var/run/snakeway.sock"
    pub sock: String,

    pub use_tls: bool,

    pub sni: String,

    pub weight: u32,
}

impl UpstreamUnixConfig {
    pub fn new(sock: String, use_tls: bool, weight: u32) -> Self {
        Self {
            sock,
            use_tls,
            sni: "localhost".to_string(),
            weight,
        }
    }
}
