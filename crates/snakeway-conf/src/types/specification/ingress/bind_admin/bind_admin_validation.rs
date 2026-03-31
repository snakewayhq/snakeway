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

#[cfg(test)]
mod tests {
    use crate::types::{BindAdminSpec, BindInterfaceInput, Origin, TlsTerminationSpec};
    use crate::validation::{ValidateSpec, ValidationReport};
    use rcgen::generate_simple_self_signed;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn invalid_bind_addr() {
        // Arrange
        let mut report = ValidationReport::default();
        let bind_admin = BindAdminSpec {
            interface: BindInterfaceInput::Keyword("bad-addr".to_string()),
            port: 9000,
            ..Default::default()
        };
        let origin = Origin::test("bind_admin");

        // Act
        bind_admin.validate(&origin, &mut report);

        // Assert
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.message == "invalid bind address: bad-addr")
        );
    }

    #[test]
    fn cannot_bind_to_all_interfaces() {
        // Arrange
        let mut report = ValidationReport::default();
        let dir = tempdir().expect("failed to create temp dir");

        let cert = generate_simple_self_signed(vec!["localhost".into()])
            .expect("failed to generate self-signed cert");
        let cert_pem = cert.cert.pem();
        let key_pem = cert.signing_key.serialize_pem();

        let cert_path = dir.path().join("tmp-cert.pem");
        let mut cert_file = File::create(&cert_path).expect("failed to create cert file");
        cert_file
            .write_all(cert_pem.as_bytes())
            .expect("failed to write cert");

        let key_path = dir.path().join("tmp-key.pem");
        let mut key_file = File::create(&key_path).expect("failed to create key file");
        key_file
            .write_all(key_pem.as_bytes())
            .expect("failed to write key");

        let bind_admin = BindAdminSpec {
            interface: BindInterfaceInput::Keyword("all".to_string()),
            port: 9000,
            tls: TlsTerminationSpec::Manual {
                cert: cert_path,
                key: key_path,
            },
            ..Default::default()
        };
        let origin = Origin::test("bind_admin");

        // Act
        bind_admin.validate(&origin, &mut report);

        // Assert
        assert_eq!(report.errors.len(), 1);
        assert_eq!(
            report.errors[0].message,
            "admin API cannot bind to all interfaces"
        );
        assert_eq!(
            report.errors[0].help.as_deref(),
            Some("Use loopback or a specific IP address.")
        );
    }
}
