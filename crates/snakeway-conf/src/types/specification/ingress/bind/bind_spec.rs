use crate::resolution::ResolveError;
use crate::types::{
    BindInterfaceSpec, ConnectionRateLimitingFilterSpec, Http2Spec, NetworkConnectionFilterSpec,
    RedirectSpec, TlsTerminationSpec,
};
use crate::validation::ConfigError;
use confval::prelude::{Located, Report, Validate, range_constraint};
use serde::Serialize;
use std::net::SocketAddr;

range_constraint!(PORT, i64, min: 1, max: 65535);
range_constraint!(RESPONSE_CODE, i64, min: 300, max: 399);

#[derive(Debug, Serialize, Default, Clone, confval::Spec)]
pub struct BindSpec {
    pub interface: Located<String>,
    #[confval[range = PORT]]
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

        Ok(SocketAddr::new(interface.as_ip(), self.port.value as u16))
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

        // Interface validation.
        let interface: Result<BindInterfaceSpec, _> = self.interface.value.as_str().try_into();
        match interface {
            Ok(BindInterfaceSpec::Ip(ip)) if ip.is_unspecified() => {
                report
                    .error("invalid bind address: 0.0.0.0")
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
    fn bind_unspecified_ip_rejected() {
        // Arrange
        let mut report = Report::new();
        let bind = BindSpec {
            interface: Located::detached("0.0.0.0".to_string()),
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
                .any(|e| e.message.contains("invalid bind address: 0.0.0.0"))
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
