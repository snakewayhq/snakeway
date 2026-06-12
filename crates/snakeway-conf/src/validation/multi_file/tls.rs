use crate::types::{CertStoreSpec, IngressSpec, ServerSpec, TlsTerminationSpec};
use confval::provenance::{Located, Report};

pub(crate) fn validate_tls(
    server: &ServerSpec,
    ingresses: &[Located<IngressSpec>],
    report: &mut Report,
) {
    let mut any_tls_listener = false;
    let mut any_acme_listener = false;

    for ingress in ingresses {
        if let Some(bind) = &ingress.value.bind
            && let Some(certificate_spec) = &bind.value.tls
        {
            any_tls_listener = true;

            // Empty ACME domain lists are reported by the single-file pass.
            if matches!(certificate_spec.value, TlsTerminationSpec::Acme { .. }) {
                any_acme_listener = true;
            }
        }
    }

    // If ACME is configured anywhere, server.tls_automation must exist
    if any_acme_listener {
        let Some(tls_automation_cfg) = &server.tls_automation else {
            report
                .error("ACME configured in ingress but server.tls_automation is not configured")
                .emit();
            return;
        };

        match &tls_automation_cfg.value.cert_store.value {
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
                // by the server entity validation pass.
            }
        }
    }

    // Optional: warn if server.tls_automation exists but no TLS listeners
    if let Some(tls_automation) = &server.tls_automation
        && !any_tls_listener
    {
        report
            .warning("server.tls_automation configured but no TLS listeners defined")
            .at(tls_automation.span)
            .emit();
    }
}

#[cfg(test)]
mod tests {
    use super::validate_tls;
    use crate::types::*;
    use confval::provenance::{Located, Report};
    use std::path::PathBuf;

    fn minimal_bind_with_acme() -> BindSpec {
        BindSpec {
            interface: Located::detached("loopback".to_string()),
            port: Located::detached(8443),
            tls: Some(Located::detached(TlsTerminationSpec::Acme {
                domains: vec![Located::detached("example.com".to_string())],
                challenge: Located::detached(ACME_CHALLENGE_HTTP01.to_string()),
            })),
            ..Default::default()
        }
    }

    fn minimal_tls_automation() -> TlsAutomationSpec {
        TlsAutomationSpec {
            acme: Located::detached(AcmeServerSpec {
                directory_url: Located::detached("https://acme.example.com/directory".to_string()),
                data_dir: Located::detached(PathBuf::from("/tmp/acme")),
                contact_email: vec![Located::detached("admin@example.com".to_string())],
                ca_file: None,
            }),
            cert_store: Located::detached(CertStoreSpec::Memory),
            renew_within_days: Located::detached(30),
        }
    }

    fn minimal_service() -> Located<ServiceSpec> {
        Located::detached(ServiceSpec {
            load_balancing_strategy: Located::detached("failover".to_string()),
            routes: vec![Located::detached(ServiceRouteSpec {
                path: Located::detached("/".to_string()),
                hosts: vec![Located::detached("example.com".to_string())],
                ..Default::default()
            })],
            upstreams: vec![Located::detached(UpstreamSpec {
                endpoint: Some(Located::detached(EndpointSpec {
                    host: Located::detached("127.0.0.1".to_string()),
                    port: Located::detached(8080),
                    tls: None,
                })),
                sock: None,
                weight: Located::detached(1),
            })],
            ..Default::default()
        })
    }

    fn ingress(bind: BindSpec) -> Located<IngressSpec> {
        Located::detached(IngressSpec {
            bind: Some(Located::detached(bind)),
            services: vec![minimal_service()],
            ..Default::default()
        })
    }

    #[test]
    fn acme_requires_tls_automation() {
        // Arrange
        let mut report = Report::new();
        let server = ServerSpec {
            tls_automation: None,
            ..Default::default()
        };

        // Act
        validate_tls(&server, &[ingress(minimal_bind_with_acme())], &mut report);

        // Assert
        assert!(report.issues().iter().any(|i| i.message
            == "ACME configured in ingress but server.tls_automation is not configured"));
    }

    #[test]
    fn tls_automation_without_tls_listeners_produces_warning() {
        // Arrange
        let mut report = Report::new();
        let server = ServerSpec {
            tls_automation: Some(Located::detached(minimal_tls_automation())),
            ..Default::default()
        };
        let plain_bind = BindSpec {
            interface: Located::detached("loopback".to_string()),
            port: Located::detached(8080),
            tls: None,
            ..Default::default()
        };

        // Act
        validate_tls(&server, &[ingress(plain_bind)], &mut report);

        // Assert
        assert!(!report.has_errors());
        assert!(
            report
                .issues()
                .iter()
                .any(|w| w.message
                    == "server.tls_automation configured but no TLS listeners defined")
        );
    }

    #[test]
    fn valid_acme_with_tls_automation() {
        // Arrange
        let mut report = Report::new();
        let server = ServerSpec {
            tls_automation: Some(Located::detached(minimal_tls_automation())),
            ..Default::default()
        };

        // Act
        validate_tls(&server, &[ingress(minimal_bind_with_acme())], &mut report);

        // Assert
        assert!(!report.has_issues(), "got: {:?}", report.issues());
    }
}
