use crate::conf::resolution::ResolveError;
use crate::conf::types::{EndpointSpec, EndpointTlsSpec};
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
        let protocol = if spec.tls.is_some() { "https" } else { "http" };
        let addr = spec.resolve()?;
        let maybe_tls_config: Option<UpstreamTlsConfig> = spec.tls.clone().map(|tls| tls.into());
        Ok(Self {
            weight,
            tls: maybe_tls_config,
            url: format!("{protocol}://{addr}"),
        })
    }
}

/// Represent TLS settings for origin server connections.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpstreamTlsConfig {
    pub sni: String,
    pub verify: bool,
    pub ca_file: Option<PathBuf>,
}

impl From<EndpointTlsSpec> for UpstreamTlsConfig {
    fn from(spec: EndpointTlsSpec) -> Self {
        Self {
            sni: spec.sni,
            verify: spec.verify,
            ca_file: spec.ca_file,
        }
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
