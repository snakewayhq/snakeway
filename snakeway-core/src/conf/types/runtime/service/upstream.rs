use crate::conf::resolution::ResolveError;
use crate::conf::types::{EndpointSpec, EndpointTlsSpec};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct UpstreamTcpConfig {
    /// e.g. "http://10.0.0.1:8080"
    pub(crate) url: String,

    pub(crate) weight: u32,

    pub(crate) tls: Option<UpstreamTlsConfig>,
}

impl UpstreamTcpConfig {
    pub(crate) fn new(weight: u32, spec: &EndpointSpec) -> Result<Self, ResolveError> {
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
pub(crate) struct UpstreamTlsConfig {
    pub(crate) sni: String,
    pub(crate) verify: bool,
    pub(crate) ca_file: Option<PathBuf>,
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
pub(crate) struct UpstreamUnixConfig {
    /// e.g. "/var/run/snakeway.sock"
    pub(crate) sock: String,

    pub(crate) use_tls: bool,

    pub(crate) sni: String,

    pub(crate) weight: u32,
}

impl UpstreamUnixConfig {
    pub(crate) fn new(sock: String, use_tls: bool, weight: u32) -> Self {
        Self {
            sock,
            use_tls,
            sni: "localhost".to_string(),
            weight,
        }
    }
}
