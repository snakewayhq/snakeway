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

/// Represent TLS settings for origin server connections.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpstreamTlsConfig {
    pub sni: String,
    pub verify: bool,
    pub ca_file: Option<PathBuf>,
}

impl From<&EndpointTlsSpec> for UpstreamTlsConfig {
    fn from(spec: &EndpointTlsSpec) -> Self {
        Self {
            sni: spec.sni.value.clone(),
            verify: spec.verify.value,
            ca_file: spec.ca_file.as_ref().map(|p| p.value.clone()),
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

/// Build an upstream config without performing DNS resolution.
///
/// Hostnames are preserved in the URL so that Pingora resolves them lazily
/// at connection time. This avoids blocking startup when upstream DNS
/// entries are not yet available (e.g., container orchestration scenarios).
impl UpstreamTcpConfig {
    pub fn new(weight: u32, spec: &EndpointSpec) -> Self {
        let protocol = if spec.tls.is_some() { "https" } else { "http" };
        let tls = spec.tls.as_ref().map(|t| (&t.value).into());

        // IPv6 addresses need brackets in URLs.
        let host_str = match HostSpec::parse(&spec.host.value) {
            HostSpec::Ip(IpAddr::V6(v6)) => format!("[{v6}]"),
            other => other.to_string(),
        };

        Self {
            weight,
            tls,
            url: format!("{protocol}://{host_str}:{}", spec.port.value),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_config_from_spec() {
        // Arrange
        use confval::provenance::Located;
        let spec = EndpointTlsSpec {
            sni: Located::detached("backend.internal".to_string()),
            verify: Located::detached(true),
            ca_file: Some(Located::detached(PathBuf::from("/etc/ssl/ca.pem"))),
        };

        // Act
        let config: UpstreamTlsConfig = (&spec).into();

        // Assert
        assert_eq!(config.sni, "backend.internal");
        assert!(config.verify);
        assert_eq!(config.ca_file, Some(PathBuf::from("/etc/ssl/ca.pem")));
    }

    #[test]
    fn tcp_upstream_http_url() {
        // Arrange
        let spec = EndpointSpec {
            host: confval::provenance::Located::detached("127.0.0.1".to_string()),
            port: confval::provenance::Located::detached(3000),
            tls: None,
        };

        // Act
        let config = UpstreamTcpConfig::new(1, &spec);

        // Assert
        assert_eq!(config.url, "http://127.0.0.1:3000");
        assert!(config.tls.is_none());
    }

    #[test]
    fn tcp_upstream_https_url() {
        // Arrange
        let spec = EndpointSpec {
            host: confval::provenance::Located::detached("127.0.0.1".to_string()),
            port: confval::provenance::Located::detached(3000),
            tls: Some(confval::provenance::Located::detached(EndpointTlsSpec {
                sni: confval::provenance::Located::detached("example.com".to_string()),
                verify: confval::provenance::Located::detached(true),
                ca_file: None,
            })),
        };

        // Act
        let config = UpstreamTcpConfig::new(1, &spec);

        // Assert
        assert_eq!(config.url, "https://127.0.0.1:3000");
        assert!(config.tls.is_some());
    }

    #[test]
    fn tcp_upstream_ipv6_bracketed() {
        // Arrange
        let spec = EndpointSpec {
            host: confval::provenance::Located::detached("::1".to_string()),
            port: confval::provenance::Located::detached(3000),
            tls: None,
        };

        // Act
        let config = UpstreamTcpConfig::new(1, &spec);

        // Assert
        assert_eq!(config.url, "http://[::1]:3000");
    }

    #[test]
    fn tcp_upstream_hostname_preserved() {
        // Arrange
        let spec = EndpointSpec {
            host: confval::provenance::Located::detached("my-service".to_string()),
            port: confval::provenance::Located::detached(3000),
            tls: None,
        };

        // Act
        let config = UpstreamTcpConfig::new(1, &spec);

        // Assert
        assert_eq!(config.url, "http://my-service:3000");
    }

    #[test]
    fn unix_upstream_sets_localhost_sni() {
        // Arrange / Act
        let config = UpstreamUnixConfig::new("/var/run/app.sock".to_string(), false, 1);

        // Assert
        assert_eq!(config.sni, "localhost");
        assert_eq!(config.sock, "/var/run/app.sock");
        assert!(!config.use_tls);
        assert_eq!(config.weight, 1);
    }
}
