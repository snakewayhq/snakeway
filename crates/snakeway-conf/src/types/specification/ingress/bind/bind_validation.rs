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
            report.http2_requires_tls(&self.interface.to_string(), &origin);
        }

        // Redirect HTTP to HTTPS validation.
        if let Some(redirect) = &self.redirect_http_to_https {
            redirect.validate(origin, report);
        }

        if let Some(redirect) = &self.redirect_http_to_https {
            if self.tls.is_none() {
                report.redirect_http_to_https_requires_tls(&self.interface.to_string(), &origin);
            }
        }

        // Interface validation.
        let interface: Result<BindInterfaceSpec, _> = self.interface.clone().try_into();
        match interface {
            Ok(BindInterfaceSpec::Ip(ip)) if ip.is_unspecified() => {
                report.invalid_bind_addr("0.0.0.0", &origin);
            }
            Ok(_) => {
                // All good.
            }
            Err(_) => {
                report.invalid_bind_addr(&self.interface.to_string(), &origin);
            }
        }
    }
}
