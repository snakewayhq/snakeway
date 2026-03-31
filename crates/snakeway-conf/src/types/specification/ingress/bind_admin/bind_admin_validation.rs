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

        let maybe_interface: Result<BindInterfaceSpec, _> = self.interface.clone().try_into();

        match maybe_interface {
            Ok(BindInterfaceSpec::Ip(ip)) if ip.is_unspecified() => {
                // Note: an unspecified IP address is "0.0.0.0" or "::"
                // which resolves to all interfaces.
                // Binding to all interfaces exposes the admin API to the network,
                // which is not allowed.
                report.invalid_bind_addr("0.0.0.0 or ::", &origin);
            }
            Ok(_) => {
                // All good.
            }
            Err(_) => {
                report.invalid_bind_addr(&self.interface.to_string(), &origin);
                return;
            }
        }

        if let Ok(interface) = maybe_interface
            && matches!(interface, BindInterfaceSpec::All)
        {
            // This check might look redundant, but it's not.
            // There are two ways to bind to all interfaces:
            // 1. Use "all" enum option.
            // 2. Use a specific IP address.
            report.error(
                "admin API cannot bind to all interfaces".to_string(),
                &origin,
                Some("Use loopback or a specific IP address.".to_string()),
            );
        }
    }
}
