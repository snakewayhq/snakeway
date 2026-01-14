use crate::conf::resolution::ResolveError;
use crate::conf::types::EndpointSpec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpstreamTcpConfig {
    /// e.g. "http://10.0.0.1:8080"
    pub url: String,

    pub weight: u32,
}

impl UpstreamTcpConfig {
    pub fn new(use_tls: bool, weight: u32, spec: &EndpointSpec) -> Result<Self, ResolveError> {
        let protocol = if use_tls { "https" } else { "http" };
        let addr = spec.resolve()?;
        Ok(Self {
            weight,
            url: format!("{protocol}://{addr}"),
        })
    }
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
