use crate::types::{EndpointSpec, EndpointTlsSpec, HostSpec};
use o2o::o2o;
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
#[derive(o2o, Debug, Clone, Deserialize, Serialize)]
#[from_owned(EndpointTlsSpec)]
pub struct UpstreamTlsConfig {
    pub sni: String,
    pub verify: bool,
    pub ca_file: Option<PathBuf>,
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
        let spec = EndpointTlsSpec {
            sni: "backend.internal".to_string(),
            verify: true,
            ca_file: Some(PathBuf::from("/etc/ssl/ca.pem")),
        };

        // Act
        let config: UpstreamTlsConfig = spec.into();

        // Assert
        assert_eq!(config.sni, "backend.internal");
        assert!(config.verify);
        assert_eq!(config.ca_file, Some(PathBuf::from("/etc/ssl/ca.pem")));
    }

    #[test]
    fn tcp_upstream_http_url() {
        // Arrange
        let spec = EndpointSpec {
            host: HostSpec::Ip("127.0.0.1".parse().unwrap()),
            port: 3000,
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
            host: HostSpec::Ip("127.0.0.1".parse().unwrap()),
            port: 3000,
            tls: Some(EndpointTlsSpec {
                sni: "example.com".to_string(),
                verify: true,
                ca_file: None,
            }),
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
            host: HostSpec::Ip("::1".parse().unwrap()),
            port: 3000,
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
            host: HostSpec::Hostname("my-service".to_string()),
            port: 3000,
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
