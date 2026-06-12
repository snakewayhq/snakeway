use super::redirect_spec::{report_invalid_port, validate_redirect};
use super::tls_termination_spec::validate_tls_termination;
use crate::resolution::ResolveError;
use crate::types::{
    BindInterfaceSpec, ConnectionRateLimitingFilterSpec, HclInt, NetworkConnectionFilterSpec,
    RedirectSpec, TlsTerminationSpec,
};
use crate::validation::ConfigError;
use crate::validation::validator::is_valid_port;
use confval::provenance::{Located, Report};
use serde::Serialize;
use std::net::SocketAddr;

use super::connection_rate_limiting_filter_spec::validate_connection_rate_limiting_filter;
use super::network_connection_filter_spec::validate_network_connection_filter;

#[derive(Debug, Serialize, Default, Clone, confval::Spec)]
pub struct BindSpec {
    pub interface: Located<String>,
    pub port: Located<HclInt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[confval(nested)]
    pub tls: Option<Located<TlsTerminationSpec>>,
    #[confval(default)]
    pub enable_http2: Located<bool>,
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

pub fn validate_bind(spec: &BindSpec, report: &mut Report) {
    // Port validation.
    if !is_valid_port(spec.port.value) {
        report_invalid_port(&spec.port, report);
    }

    // Connection filters.
    if let Some(connection_filter) = &spec.connection_filter {
        validate_network_connection_filter(&connection_filter.value, report);
    }

    if let Some(connection_rate_limiting_filter) = &spec.connection_rate_limiting_filter {
        validate_connection_rate_limiting_filter(&connection_rate_limiting_filter.value, report);
    }

    // TLS cert/key/acme validation.
    if let Some(tls) = &spec.tls {
        validate_tls_termination(&tls.value, report);
    }

    // HTTP/2 requires TLS.
    if spec.enable_http2.value && spec.tls.is_none() {
        report
            .error(format!("HTTP/2 requires TLS: {}", spec.interface.value))
            .at(spec.enable_http2.span)
            .help("Enable TLS on the bind or disable HTTP/2.")
            .emit();
    }

    // Redirect HTTP to HTTPS validation.
    if let Some(redirect) = &spec.redirect_http_to_https {
        validate_redirect(&redirect.value, report);
    }

    // Redirect HTTP to HTTPS requires TLS.
    if let Some(redirect) = &spec.redirect_http_to_https
        && spec.tls.is_none()
    {
        report
            .error(format!(
                "redirect_http_to_https requires TLS: {}",
                spec.interface.value
            ))
            .at(redirect.span)
            .help("Enable TLS on the bind or remove redirect_http_to_https.")
            .emit();
    }

    // Interface validation.
    let interface: Result<BindInterfaceSpec, _> = spec.interface.value.as_str().try_into();
    match interface {
        Ok(BindInterfaceSpec::Ip(ip)) if ip.is_unspecified() => {
            report
                .error("invalid bind address: 0.0.0.0")
                .at(spec.interface.span)
                .emit();
        }
        Ok(_) => {
            // All good.
        }
        Err(_) => {
            report
                .error(format!("invalid bind address: {}", spec.interface.value))
                .at(spec.interface.span)
                .emit();
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
        validate_bind(&bind, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .issues()
                .iter()
                .any(|e| e.message.contains("invalid port: 0"))
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
        validate_bind(&bind, &mut report);

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
        validate_bind(&bind, &mut report);

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
        validate_bind(&bind, &mut report);

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
        validate_bind(&bind, &mut report);

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
        validate_bind(&bind, &mut report);

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
