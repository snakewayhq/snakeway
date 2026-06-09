use super::bind_issues;
use crate::types::{BindInterfaceSpec, BindSpec, HclOrigin};
use crate::validation::validator::is_valid_port;
use confval::{ValidateSpec, ValidationReport};

impl ValidateSpec<HclOrigin> for BindSpec {
    fn validate(&self, origin: &HclOrigin, report: &mut ValidationReport<HclOrigin>) {
        // Port validation.
        if !is_valid_port(self.port) {
            report.push(bind_issues::invalid_port(self.port, origin));
        }

        // Connection filters.
        if let Some(connection_filter) = &self.connection_filter {
            connection_filter.validate(origin, report);
        }

        if let Some(connection_rate_limiting_filter) = &self.connection_rate_limiting_filter {
            connection_rate_limiting_filter.validate(origin, report);
        }

        // TLS cert/key/acme validation.
        if let Some(tls) = &self.tls {
            tls.validate(origin, report);
        }

        // HTTP/2 requires TLS.
        if self.enable_http2 && self.tls.is_none() {
            report.push(bind_issues::http2_requires_tls(
                &self.interface.to_string(),
                origin,
            ));
        }

        // HTTP/2 tuning options.
        if let Some(http2) = &self.http2 {
            if !self.enable_http2 {
                report.push(bind_issues::http2_options_require_enable_http2(origin));
            }
            http2.validate(origin, report);
        }

        // Redirect HTTP to HTTPS validation.
        if let Some(redirect) = &self.redirect_http_to_https {
            redirect.validate(origin, report);
        }

        // Redirect HTTP to HTTPS requires TLS.
        if self.redirect_http_to_https.is_some() && self.tls.is_none() {
            report.push(bind_issues::redirect_http_to_https_requires_tls(
                &self.interface.to_string(),
                origin,
            ));
        }

        // Interface validation.
        let interface: Result<BindInterfaceSpec, _> = self.interface.clone().try_into();
        match interface {
            Ok(BindInterfaceSpec::Ip(ip)) if ip.is_unspecified() => {
                report.push(bind_issues::invalid_bind_addr("0.0.0.0", origin));
            }
            Ok(_) => {
                // All good.
            }
            Err(_) => {
                report.push(bind_issues::invalid_bind_addr(
                    &self.interface.to_string(),
                    origin,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{
        BindInterfaceInput, BindSpec, HclOrigin, Http2Spec, RedirectSpec, TlsTerminationSpec,
    };
    use confval::{ValidateSpec, ValidationReport};

    fn minimal_bind() -> BindSpec {
        BindSpec {
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 8080,
            ..Default::default()
        }
    }

    #[test]
    fn bind_port_zero_rejected() {
        // Arrange
        let mut report = ValidationReport::default();
        let bind = BindSpec {
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 0,
            ..Default::default()
        };
        let origin = HclOrigin::test("bind");

        // Act
        bind.validate(&origin, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .errors()
                .iter()
                .any(|e| e.message.contains("invalid port: 0"))
        );
    }

    #[test]
    fn bind_unspecified_ip_rejected() {
        // Arrange
        let mut report = ValidationReport::default();
        let bind = BindSpec {
            interface: BindInterfaceInput::Keyword("0.0.0.0".to_string()),
            port: 8080,
            ..Default::default()
        };
        let origin = HclOrigin::test("bind");

        // Act
        bind.validate(&origin, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .errors()
                .iter()
                .any(|e| e.message.contains("invalid bind address: 0.0.0.0"))
        );
    }

    #[test]
    fn bind_invalid_interface_rejected() {
        // Arrange
        let mut report = ValidationReport::default();
        let bind = BindSpec {
            interface: BindInterfaceInput::Keyword("bad-keyword".to_string()),
            port: 8080,
            ..Default::default()
        };
        let origin = HclOrigin::test("bind");

        // Act
        bind.validate(&origin, &mut report);

        // Assert
        assert!(report.has_issues());
        assert!(
            report
                .errors()
                .iter()
                .any(|e| e.message.contains("invalid bind address"))
        );
    }

    #[test]
    fn valid_minimal_bind() {
        // Arrange
        let mut report = ValidationReport::default();
        let bind = minimal_bind();
        let origin = HclOrigin::test("bind");

        // Act
        bind.validate(&origin, &mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn http2_requires_tls() {
        // Arrange
        let mut report = ValidationReport::default();
        let mut bind = minimal_bind();
        bind.enable_http2 = true;
        let origin = HclOrigin::test("bind");

        // Act
        bind.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors()[0].message, "HTTP/2 requires TLS: loopback");
        assert_eq!(
            report.errors()[0].help.as_deref(),
            Some("Enable TLS on the bind or disable HTTP/2.")
        );
    }

    #[test]
    fn http2_options_without_enable_http2_rejected() {
        // Arrange
        let mut report = ValidationReport::default();
        let mut bind = minimal_bind();
        bind.http2 = Some(Http2Spec::default());
        let origin = HclOrigin::test("bind");

        // Act
        bind.validate(&origin, &mut report);

        // Assert
        assert_eq!(
            report.errors()[0].message,
            "http2 settings require enable_http2 = true"
        );
        assert_eq!(
            report.errors()[0].help.as_deref(),
            Some("Set enable_http2 = true or remove the http2 block.")
        );
    }

    #[test]
    fn http2_options_with_enable_http2_accepted() {
        // Arrange
        let mut report = ValidationReport::default();
        let mut bind = minimal_bind();
        bind.enable_http2 = true;
        bind.tls = Some(TlsTerminationSpec::Acme {
            domains: vec!["example.com".to_string()],
            challenge: Default::default(),
        });
        bind.http2 = Some(Http2Spec {
            max_concurrent_streams: Some(200),
            max_header_list_size: Some(65536),
            initial_window_size: Some(65535),
            initial_connection_window_size: Some(1048576),
        });
        let origin = HclOrigin::test("bind");

        // Act
        bind.validate(&origin, &mut report);

        // Assert
        assert!(!report.has_issues());
    }

    #[test]
    fn http2_options_out_of_range_field_rejected() {
        // Arrange
        let mut report = ValidationReport::default();
        let mut bind = minimal_bind();
        bind.enable_http2 = true;
        bind.tls = Some(TlsTerminationSpec::Acme {
            domains: vec!["example.com".to_string()],
            challenge: Default::default(),
        });
        bind.http2 = Some(Http2Spec {
            initial_window_size: Some(2_147_483_648),
            ..Default::default()
        });
        let origin = HclOrigin::test("bind");

        // Act
        bind.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors().len(), 1);
        assert!(report.errors()[0].message.contains("initial_window_size"));
    }

    #[test]
    fn redirect_should_not_exist_without_tls() {
        // Arrange
        let mut report = ValidationReport::default();
        let mut bind = minimal_bind();
        bind.redirect_http_to_https = Some(RedirectSpec {
            port: 8080,
            status: 308,
        });
        let origin = HclOrigin::test("bind");

        // Act
        bind.validate(&origin, &mut report);

        // Assert
        assert_eq!(
            report.errors()[0].message,
            "redirect_http_to_https requires TLS: loopback"
        );
        assert_eq!(
            report.errors()[0].help.as_deref(),
            Some("Enable TLS on the bind or remove redirect_http_to_https.")
        );
    }
}
