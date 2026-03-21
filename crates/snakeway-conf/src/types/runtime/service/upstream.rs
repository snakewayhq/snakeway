use crate::types::{EndpointSpec, EndpointTlsSpec, HostSpec};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpstreamTcpConfig {
    /// e.g. "http://my-service:8080" or "http://10.0.0.1:8080"
    pub url: String,

    pub weight: u32,

    pub tls: Option<UpstreamTlsConfig>,
}

/// Build an upstream config without performing DNS resolution.
///
/// Hostnames are preserved in the URL so that Pingora resolves them lazily
/// at connection time. This avoids blocking startup when upstream DNS
/// entries are not yet available (e.g., container orchestration scenarios).
impl UpstreamTcpConfig {
    pub fn new(weight: u32, spec: &EndpointSpec) -> Self {
        let protocol = if spec.tls.is_some() { "https" } else { "http" };
        let tls = spec.tls.clone().map(|t| t.into());

        // IPv6 addresses need brackets in URLs.
        let host_str = match &spec.host {
            HostSpec::Ip(IpAddr::V6(v6)) => format!("[{v6}]"),
            other => other.to_string(),
        };

        Self {
            weight,
            tls,
            url: format!("{protocol}://{host_str}:{}", spec.port),
        }
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
