use crate::types::{BindInterfaceSpec, BindSpec, Origin};
use crate::validation::validator::is_valid_port;
use crate::validation::{ValidateSpec, ValidationReport};

impl ValidateSpec for BindSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        // Port validation.
        if !is_valid_port(self.port) {
            report.invalid_port(self.port, origin);
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
            report.http2_requires_tls(&self.interface.to_string(), origin);
        }

        // Redirect HTTP to HTTPS validation.
        if let Some(redirect) = &self.redirect_http_to_https {
            redirect.validate(origin, report);
        }

        // Redirect HTTP to HTTPS requires TLS.
        if self.redirect_http_to_https.is_some() && self.tls.is_none() {
            report.redirect_http_to_https_requires_tls(&self.interface.to_string(), origin);
        }

        // Interface validation.
        let interface: Result<BindInterfaceSpec, _> = self.interface.clone().try_into();
        match interface {
            Ok(BindInterfaceSpec::Ip(ip)) if ip.is_unspecified() => {
                report.invalid_bind_addr("0.0.0.0", origin);
            }
            Ok(_) => {
                // All good.
            }
            Err(_) => {
                report.invalid_bind_addr(&self.interface.to_string(), origin);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{BindInterfaceInput, BindSpec, Origin, RedirectSpec};
    use crate::validation::{ValidateSpec, ValidationReport};

    fn minimal_bind() -> BindSpec {
        BindSpec {
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 8080,
            ..Default::default()
        }
    }

    #[test]
    fn valid_minimal_bind() {
        // Arrange
        let mut report = ValidationReport::default();
        let bind = minimal_bind();
        let origin = Origin::test("bind");

        // Act
        bind.validate(&origin, &mut report);

        // Assert
        assert!(!report.has_violations());
    }

    #[test]
    fn http2_requires_tls() {
        // Arrange
        let mut report = ValidationReport::default();
        let mut bind = minimal_bind();
        bind.enable_http2 = true;
        let origin = Origin::test("bind");

        // Act
        bind.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors[0].message, "HTTP/2 requires TLS: loopback");
        assert_eq!(
            report.errors[0].help.as_deref(),
            Some("Enable TLS on the bind or disable HTTP/2.")
        );
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
        let origin = Origin::test("bind");

        // Act
        bind.validate(&origin, &mut report);

        // Assert
        assert_eq!(
            report.errors[0].message,
            "redirect_http_to_https requires TLS: loopback"
        );
        assert_eq!(
            report.errors[0].help.as_deref(),
            Some("Enable TLS on the bind or remove redirect_http_to_https.")
        );
    }
}
