use crate::types::EndpointTlsSpec;
use o2o::o2o;
use serde::{Deserialize, Serialize};
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
}
