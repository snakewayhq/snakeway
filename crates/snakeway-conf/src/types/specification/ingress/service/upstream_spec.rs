use crate::resolution::ResolveError;
use crate::types::HclInt;
use crate::types::specification::ingress::bind::report_invalid_port;
use crate::validation::validator::{is_valid_hostname, is_valid_port, validate_cert_pem};
use confval::prelude::{Located, Report, Validate};
use serde::Serialize;
use std::fmt;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::path::PathBuf;

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct UpstreamSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub endpoint: Option<Located<EndpointSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sock: Option<Located<String>>,
    #[confval(default = 1)]
    pub weight: Located<HclInt>,
}

#[derive(Debug, Serialize, Clone, Default, confval::Spec)]
pub struct EndpointSpec {
    pub host: Located<String>,
    pub port: Located<HclInt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub tls: Option<Located<EndpointTlsSpec>>,
}

#[derive(Debug, Serialize, Clone, Default, confval::Spec)]
pub struct EndpointTlsSpec {
    pub sni: Located<String>,
    pub verify: Located<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_file: Option<Located<PathBuf>>,
}

/// Classification of an endpoint host string, used by validation and
/// lowering. The Spec stores the raw string; this is the parsed view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostSpec {
    Ip(IpAddr),
    Hostname(String),
}

impl HostSpec {
    pub fn parse(host: &str) -> HostSpec {
        match host.parse::<IpAddr>() {
            Ok(ip) => HostSpec::Ip(ip),
            Err(_) => HostSpec::Hostname(host.to_string()),
        }
    }
}

impl fmt::Display for HostSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostSpec::Ip(ip) => write!(f, "{ip}"),
            HostSpec::Hostname(name) => write!(f, "{}", name.to_lowercase()),
        }
    }
}

impl EndpointSpec {
    pub fn resolve(&self) -> Result<SocketAddr, ResolveError> {
        let port = self.port.value as u16;
        let ip = match HostSpec::parse(&self.host.value) {
            HostSpec::Ip(ip) => ip,
            HostSpec::Hostname(name) => {
                let mut addrs = (name.as_str(), port)
                    .to_socket_addrs()
                    .map_err(|_| ResolveError::DnsFailed(name.clone()))?;

                addrs
                    .next()
                    .ok_or_else(|| ResolveError::NoAddresses(name.clone()))?
                    .ip()
            }
        };

        Ok(SocketAddr::new(ip, port))
    }
}

impl Validate for UpstreamSpec {
    fn validate(&self, report: &mut Report) {
        if self.weight.value == 0 || self.weight.value > 1_000 {
            report
                .error(format!("invalid upstream weight: {}", self.weight.value))
                .at(self.weight.span)
                .emit();
        }
    }
}

impl Validate for EndpointSpec {
    fn validate(&self, report: &mut Report) {
        let spec = self;
        match HostSpec::parse(&spec.host.value) {
            HostSpec::Ip(ip) if ip.is_unspecified() || ip.is_multicast() => {
                report
                    .error(format!("invalid upstream ip: {}", ip))
                    .at(spec.host.span)
                    .emit();
            }
            HostSpec::Hostname(name) if !is_valid_hostname(&name) => {
                report
                    .error(format!("invalid upstream hostname: {}", name))
                    .at(spec.host.span)
                    .emit();
            }
            _ => {}
        }

        if !is_valid_port(spec.port.value) {
            report_invalid_port(&spec.port, report);
        }
    }
}

impl Validate for EndpointTlsSpec {
    fn validate(&self, report: &mut Report) {
        let spec = self;
        if spec.sni.value.trim().is_empty() {
            report
                .error("upstream TLS SNI required")
                .at(spec.sni.span)
                .emit();
        }

        // The remaining checks describe how the certificate is verified, so
        // they mean nothing when verification is off.
        if !spec.verify.value {
            return;
        }

        if spec.sni.value.parse::<IpAddr>().is_ok() {
            report
                .error("upstream TLS SNI must be DNS name")
                .at(spec.sni.span)
                .emit();
        }

        if let Some(ca_file) = &spec.ca_file
            && let Err(e) = validate_cert_pem(&ca_file.value)
        {
            report
                .error(format!(
                    "upstream TLS has invalid CA file ({}): {}",
                    ca_file.value.to_string_lossy(),
                    e
                ))
                .at(ca_file.span)
                .emit();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn minimal_endpoint() -> EndpointSpec {
        EndpointSpec {
            host: Located::detached("127.0.0.1".to_string()),
            port: Located::detached(3000),
            tls: None,
        }
    }

    fn minimal_upstream() -> UpstreamSpec {
        UpstreamSpec {
            endpoint: Some(Located::detached(minimal_endpoint())),
            sock: None,
            weight: Located::detached(1),
        }
    }

    #[test]
    fn weight_greater_than_zero() {
        // Arrange
        let mut report = Report::new();
        let mut upstream = minimal_upstream();
        upstream.weight = Located::detached(0);

        // Act
        upstream.validate(&mut report);

        // Assert
        let error = report.issues().first().expect("expected an error");
        assert!(error.message.contains("invalid upstream weight: 0"));
    }

    #[test]
    fn weight_not_greater_than_1000() {
        // Arrange
        let mut report = Report::new();
        let mut upstream = minimal_upstream();
        upstream.weight = Located::detached(1001);

        // Act
        upstream.validate(&mut report);

        // Assert
        let error = report.issues().first().expect("expected an error");
        assert!(error.message.contains("invalid upstream weight: 1001"));
    }

    #[test]
    fn endpoint_ip_unspecified_rejected() {
        // Arrange
        let mut report = Report::new();
        let endpoint = EndpointSpec {
            host: Located::detached("0.0.0.0".to_string()),
            ..minimal_endpoint()
        };

        // Act
        endpoint.validate_all(&mut report);

        // Assert
        let error = report.issues().first().expect("expected an error");
        assert!(error.message.contains("invalid upstream ip: 0.0.0.0"));
    }

    #[test]
    fn endpoint_ip_multicast_rejected() {
        // Arrange
        let mut report = Report::new();
        let endpoint = EndpointSpec {
            host: Located::detached("224.0.0.1".to_string()),
            ..minimal_endpoint()
        };

        // Act
        endpoint.validate_all(&mut report);

        // Assert
        let error = report.issues().first().expect("expected an error");
        assert!(error.message.contains("invalid upstream ip: 224.0.0.1"));
    }

    #[test]
    fn endpoint_invalid_hostname_rejected() {
        // Arrange
        let mut report = Report::new();
        let endpoint = EndpointSpec {
            host: Located::detached("-invalid".to_string()),
            ..minimal_endpoint()
        };

        // Act
        endpoint.validate_all(&mut report);

        // Assert
        let error = report.issues().first().expect("expected an error");
        assert!(
            error
                .message
                .contains("invalid upstream hostname: -invalid")
        );
    }

    #[test]
    fn endpoint_invalid_port_rejected() {
        // Arrange
        let mut report = Report::new();
        let endpoint = EndpointSpec {
            port: Located::detached(0),
            ..minimal_endpoint()
        };

        // Act
        endpoint.validate_all(&mut report);

        // Assert
        let error = report.issues().first().expect("expected an error");
        assert!(error.message.contains("invalid port: 0"));
    }

    #[test]
    fn endpoint_empty_sni_rejected() {
        // Arrange
        let mut report = Report::new();
        let endpoint = EndpointSpec {
            tls: Some(Located::detached(EndpointTlsSpec {
                sni: Located::detached("  ".to_string()),
                verify: Located::detached(false),
                ca_file: None,
            })),
            ..minimal_endpoint()
        };

        // Act
        endpoint.validate_all(&mut report);

        // Assert
        let error = report.issues().first().expect("expected an error");
        assert_eq!(error.message, "upstream TLS SNI required");
    }

    #[test]
    fn sni_as_ip_rejected_when_verify_true() {
        // Arrange
        let mut report = Report::new();
        let tls = EndpointTlsSpec {
            sni: Located::detached("127.0.0.1".to_string()),
            verify: Located::detached(true),
            ca_file: None,
        };

        // Act
        tls.validate(&mut report);

        // Assert
        let error = report.issues().first().expect("expected an error");
        assert_eq!(error.message, "upstream TLS SNI must be DNS name");
    }

    #[test]
    fn host_classification() {
        // Arrange / Act / Assert
        assert_eq!(
            HostSpec::parse("127.0.0.1"),
            HostSpec::Ip("127.0.0.1".parse().unwrap())
        );
        assert_eq!(
            HostSpec::parse("my-service"),
            HostSpec::Hostname("my-service".to_string())
        );
    }
}
