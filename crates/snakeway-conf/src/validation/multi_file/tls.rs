use crate::types::bind_issues;
use crate::types::server_issues;
use crate::types::{CertStoreSpec, HclOrigin, IngressSpec, ServerSpec, TlsTerminationSpec};
use confval::ValidationReport;

pub(crate) fn validate_tls(
    server: &ServerSpec,
    ingresses: &[IngressSpec],
    report: &mut ValidationReport<HclOrigin>,
) {
    let mut any_tls_listener = false;
    let mut any_acme_listener = false;

    for ingress in ingresses {
        if let Some(bind) = &ingress.bind
            && let Some(certificate_spec) = &bind.tls
        {
            any_tls_listener = true;

            match certificate_spec {
                TlsTerminationSpec::Manual { .. } => {
                    // no-op: already validated in single_file.
                }
                TlsTerminationSpec::Acme { domains, .. } => {
                    any_acme_listener = true;

                    // ACME requires domains
                    if domains.is_empty() {
                        report.push(bind_issues::acme_tls_requires_domains(&bind.origin));
                    }
                }
            }
        }
    }

    // If ACME is configured anywhere, server.tls_automation must exist
    if any_acme_listener {
        let Some(tls_automation_cfg) = &server.tls_automation else {
            report.push(
                server_issues::acme_configured_in_ingress_but_server_tls_not_configured(
                    &server.origin,
                ),
            );
            return;
        };

        match &tls_automation_cfg.cert_store {
            CertStoreSpec::Memory => {
                // Nothing to validate, but should drop a warning directly here.
                // Adding a warning to the report will fail validation.
                // This is kind of a gray area.
                tracing::warn!(
                    "ACME configured with memory store. Certs will be discarded on restart."
                );
            }
            CertStoreSpec::Filesystem { .. } => {
                // cert_dir validation (empty check, create-or-verify) is handled
                // by CertStoreSpec::validate in the single-file validation pass.
            }
        }
    }

    // Optional: warn if server.tls_automation exists but no TLS listeners
    if server.tls_automation.is_some() && !any_tls_listener {
        report
            .push(server_issues::warn_server_tls_configured_with_no_tls_listeners(&server.origin));
    }
}

#[cfg(test)]
mod tests {
    use super::validate_tls;
    use crate::types::*;
    use confval::ValidationReport;
    use std::net::IpAddr;
    use std::path::PathBuf;
    use std::str::FromStr;

    fn minimal_bind_with_acme() -> BindSpec {
        BindSpec {
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 8443,
            tls: Some(TlsTerminationSpec::Acme {
                domains: vec!["example.com".to_string()],
                challenge: AcmeChallengeSpec::default(),
            }),
            ..Default::default()
        }
    }

    fn minimal_tls_automation() -> TlsAutomationSpec {
        TlsAutomationSpec {
            acme: AcmeServerSpec {
                directory_url: "https://acme.example.com/directory".to_string(),
                data_dir: PathBuf::from("/tmp/acme"),
                contact_email: vec!["admin@example.com".to_string()],
                ca_file: None,
            },
            cert_store: CertStoreSpec::Memory,
            renew_within_days: 30,
        }
    }

    fn minimal_service() -> ServiceSpec {
        ServiceSpec {
            routes: vec![ServiceRouteSpec {
                path: "/".to_string(),
                hosts: vec!["example.com".to_string()],
                ..Default::default()
            }],
            upstreams: vec![UpstreamSpec {
                endpoint: Some(EndpointSpec {
                    host: HostSpec::Ip(IpAddr::from_str("127.0.0.1").unwrap()),
                    port: 8080,
                    tls: None,
                }),
                weight: 1,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn acme_requires_tls_automation() {
        // Arrange
        let mut report = ValidationReport::default();
        let server = ServerSpec {
            tls_automation: None,
            ..Default::default()
        };
        let ingress = IngressSpec {
            bind: Some(minimal_bind_with_acme()),
            services: vec![minimal_service()],
            ..Default::default()
        };

        // Act
        validate_tls(&server, &[ingress], &mut report);

        // Assert
        assert!(report.errors().iter().any(|e| e.message
            == "ACME configured in ingress but server.tls_automation is not configured"));
    }

    #[test]
    fn tls_automation_without_tls_listeners_produces_warning() {
        // Arrange
        let mut report = ValidationReport::default();
        let server = ServerSpec {
            tls_automation: Some(minimal_tls_automation()),
            ..Default::default()
        };
        let ingress = IngressSpec {
            bind: Some(BindSpec {
                interface: BindInterfaceInput::Keyword("loopback".to_string()),
                port: 8080,
                tls: None,
                ..Default::default()
            }),
            services: vec![minimal_service()],
            ..Default::default()
        };

        // Act
        validate_tls(&server, &[ingress], &mut report);

        // Assert
        assert!(report.errors().is_empty());
        assert!(
            report
                .warnings()
                .iter()
                .any(|w| w.message
                    == "server.tls_automation configured but no TLS listeners defined")
        );
    }

    #[test]
    fn valid_acme_with_tls_automation() {
        // Arrange
        let mut report = ValidationReport::default();
        let server = ServerSpec {
            tls_automation: Some(minimal_tls_automation()),
            ..Default::default()
        };
        let ingress = IngressSpec {
            bind: Some(minimal_bind_with_acme()),
            services: vec![minimal_service()],
            ..Default::default()
        };

        // Act
        validate_tls(&server, &[ingress], &mut report);

        // Assert
        assert!(report.errors().is_empty());
        assert!(report.warnings().is_empty());
    }
}
