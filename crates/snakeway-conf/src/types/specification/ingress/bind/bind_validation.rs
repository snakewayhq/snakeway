use crate::types::{BindSpec, Origin};
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

        // Redirect HTTP to HTTPS validation.
        if let Some(redirect) = &self.redirect_http_to_https {
            redirect.validate(origin, report);
        }
    }
}
