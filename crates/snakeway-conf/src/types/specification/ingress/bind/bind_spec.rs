use crate::resolution::ResolveError;
use crate::types::is_link_local;
use crate::types::{
    AcmeChallenge, BindInterfaceSpec, ConnectionRateLimitingFilterSpec, Http2Spec,
    NetworkConnectionFilterSpec, RedirectSpec, TlsTerminationSpec,
};
use crate::validation::{ConfigError, PORT};
use confval::prelude::{Located, Report, Validate};
use serde::Serialize;
use std::net::SocketAddr;

#[derive(Debug, Serialize, Default, Clone, confval::Spec)]
pub struct BindSpec {
    pub interface: Located<String>,
    #[confval(range = PORT)]
    pub port: Located<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub tls: Option<Located<TlsTerminationSpec>>,
    #[confval(default)]
    pub enable_http2: Located<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub http2: Option<Located<Http2Spec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub redirect_http_to_https: Option<Located<RedirectSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub connection_filter: Option<Located<NetworkConnectionFilterSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub connection_rate_limiting_filter: Option<Located<ConnectionRateLimitingFilterSpec>>,
}

impl BindSpec {
    pub fn resolve(&self) -> Result<SocketAddr, ResolveError> {
        let interface: BindInterfaceSpec = self
            .interface
            .value
            .as_str()
            .try_into()
            .map_err(|e: ConfigError| ResolveError::InvalidInterface(e.to_string()))?;

        let port = u16::try_from(self.port.value)
            .map_err(|_| ResolveError::InvalidPort(self.port.value))?;

        Ok(SocketAddr::new(interface.as_ip(), port))
    }
}

impl Validate for BindSpec {
    fn validate(&self, report: &mut Report) {
        // HTTP/2 requires TLS.
        if self.enable_http2.value && self.tls.is_none() {
            report
                .error(format!("HTTP/2 requires TLS: {}", self.interface.value))
                .at(self.enable_http2.span)
                .help("Enable TLS on the bind or disable HTTP/2.")
                .emit();
        }

        // Redirect HTTP to HTTPS requires TLS.
        if let Some(redirect) = &self.redirect_http_to_https
            && self.tls.is_none()
        {
            report
                .error(format!(
                    "redirect_http_to_https requires TLS: {}",
                    self.interface.value
                ))
                .at(redirect.span)
                .help("Enable TLS on the bind or remove redirect_http_to_https.")
                .emit();
        }

        // HTTP-01 needs a plain HTTP listener that answers the CA challenge,
        // and only the redirect listener serves it. Without this pairing the
        // order can never validate and the ACME loop retries forever.
        if let Some(tls) = &self.tls
            && let TlsTerminationSpec::Acme { challenge, .. } = &tls.value
            && challenge.value == AcmeChallenge::Http01.as_str()
            && self.redirect_http_to_https.is_none()
        {
            report
                .error("HTTP-01 challenge requires a redirect_http_to_https block, which serves the ACME challenge")
                .at(tls.span)
                .help("Add a redirect_http_to_https block to this bind.")
                .emit();
        }

        // Interface validation.
        let interface: Result<BindInterfaceSpec, _> = self.interface.value.as_str().try_into();
        match interface {
            Ok(BindInterfaceSpec::Ip(ip)) if ip.is_multicast() || is_link_local(&ip) => {
                report
                    .error(format!(
                        "invalid bind address: {} (multicast and link-local addresses \
                         cannot serve traffic)",
                        self.interface.value
                    ))
                    .at(self.interface.span)
                    .emit();
            }
            Ok(_) => {
                // All good.
            }
            Err(_) => {
                report
                    .error(format!("invalid bind address: {}", self.interface.value))
                    .at(self.interface.span)
                    .emit();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_bind() -> BindSpec {
        BindSpec {
            interface: Located::detached("loopback".to_string()),
            port: Located::detached(8080),
            ..Default::default()
        }
    }

    fn bind_with_http01() -> BindSpec {
        let mut bind = minimal_bind();
        bind.tls = Some(Located::detached(TlsTerminationSpec::Acme {
            domains: vec![Located::detached("example.com".to_string())],
            challenge: Located::detached(AcmeChallenge::Http01.as_str().to_string()),
        }));
        bind
    }

    #[test]
    fn http01_challenge_without_a_redirect_block_is_rejected() {
        // Arrange
        let mut report = Report::new();
        let bind = bind_with_http01();

        // Act
        bind.validate_all(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("HTTP-01 challenge requires a redirect")),
            "issues: {:?}",
            report.issues()
        );
    }

    #[test]
    fn http01_challenge_with_a_redirect_block_is_accepted() {
        // Arrange
        let mut report = Report::new();
        let mut bind = bind_with_http01();
        bind.redirect_http_to_https = Some(Located::detached(RedirectSpec {
            port: Located::detached(9090),
            status: Located::detached(308),
        }));

        // Act
        bind.validate_all(&mut report);

        // Assert: the redirect needs the same TLS, so no issue of any kind.
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    #[test]
    fn bind_port_zero_rejected() {
        // Arrange
        let mut report = Report::new();
        let bind = BindSpec {
            port: Located::detached(0),
            ..minimal_bind()
        };

        // Act
        bind.validate_all(&mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("port must be at least 1"))
        );
    }

    #[test]
    fn bind_unspecified_ip_and_the_all_keyword_are_equivalent() {
        // Arrange: the keyword and the literal resolve to the same socket, so
        // both are accepted on the public bind.
        let mut report = Report::new();
        let literal = BindSpec {
            interface: Located::detached("0.0.0.0".to_string()),
            ..minimal_bind()
        };
        let keyword = BindSpec {
            interface: Located::detached("all".to_string()),
            ..minimal_bind()
        };

        // Act
        literal.validate(&mut report);
        keyword.validate(&mut report);

        // Assert
        assert!(!report.has_issues(), "issues: {:?}", report.issues());
    }

    #[test]
    fn bind_multicast_address_rejected() {
        // Arrange
        let mut report = Report::new();
        let bind = BindSpec {
            interface: Located::detached("224.0.0.1".to_string()),
            ..minimal_bind()
        };

        // Act
        bind.validate(&mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("multicast and link-local"))
        );
    }

    #[test]
    fn bind_link_local_address_rejected() {
        // Arrange
        let mut report = Report::new();
        let bind = BindSpec {
            interface: Located::detached("169.254.1.1".to_string()),
            ..minimal_bind()
        };

        // Act
        bind.validate(&mut report);

        // Assert
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("multicast and link-local"))
        );
    }

    #[test]
    fn bind_invalid_interface_rejected() {
        // Arrange
        let mut report = Report::new();
        let bind = BindSpec {
            interface: Located::detached("bad-keyword".to_string()),
            ..minimal_bind()
        };

        // Act
        bind.validate(&mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("invalid bind address"))
        );
    }

    #[test]
    fn valid_minimal_bind() {
        // Arrange
        let mut report = Report::new();
        let bind = minimal_bind();

        // Act
        bind.validate(&mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn http2_requires_tls() {
        // Arrange
        let mut report = Report::new();
        let mut bind = minimal_bind();
        bind.enable_http2 = Located::detached(true);

        // Act
        bind.validate(&mut report);

        // Assert
        assert_eq!(report.issues()[0].message, "HTTP/2 requires TLS: loopback");
        assert_eq!(
            report.issues()[0].help.as_deref(),
            Some("Enable TLS on the bind or disable HTTP/2.")
        );
    }

    #[test]
    fn redirect_should_not_exist_without_tls() {
        // Arrange
        let mut report = Report::new();
        let mut bind = minimal_bind();
        bind.redirect_http_to_https = Some(Located::detached(RedirectSpec {
            port: Located::detached(8080),
            status: Located::detached(308),
        }));

        // Act
        bind.validate(&mut report);

        // Assert
        assert_eq!(
            report.issues()[0].message,
            "redirect_http_to_https requires TLS: loopback"
        );
        assert_eq!(
            report.issues()[0].help.as_deref(),
            Some("Enable TLS on the bind or remove redirect_http_to_https.")
        );
    }

    #[test]
    fn resolve_loopback() {
        // Arrange
        let bind = minimal_bind();

        // Act
        let addr = bind.resolve().unwrap();

        // Assert
        assert_eq!(addr.to_string(), "127.0.0.1:8080");
    }
}
