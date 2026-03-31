use crate::types::{BindAdminSpec, BindInterfaceSpec, Origin, TlsTerminationSpec};
use crate::validation::validator::is_valid_port;
use crate::validation::{ValidateSpec, ValidationReport};

impl ValidateSpec for BindAdminSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        // Port validation.
        if !is_valid_port(self.port) {
            report.invalid_port(self.port, origin);
        }

        // TLS cert/key/acme validation.
        match &self.tls {
            TlsTerminationSpec::Manual { .. } => {
                self.tls.validate(&origin, report);
            }
            TlsTerminationSpec::Acme { .. } => {
                report.admin_bind_does_not_support_acme(&origin);
            }
        }

        let bind_interface: BindInterfaceSpec = match self.interface.clone().try_into() {
            Ok(i) => i,
            Err(_) => {
                report.invalid_bind_addr(&self.interface.to_string(), &origin);
                return;
            }
        };

        match bind_interface {
            Ok(BindInterfaceSpec::Ip(ip)) if ip.is_unspecified() => {
                report.invalid_bind_addr("0.0.0.0", &origin);
            }
            Ok(_) => {
                // All good.
            }
            Err(_) => {
                report.invalid_bind_addr(&self.interface.to_string(), &origin);
                return;
            }
        }

        if matches!(bind_interface, BindInterfaceSpec::All) {
            report.error(
                "admin API cannot bind to all interfaces".to_string(),
                &origin,
                Some("Use loopback or a specific IP address.".to_string()),
            );
        }
    }
}
