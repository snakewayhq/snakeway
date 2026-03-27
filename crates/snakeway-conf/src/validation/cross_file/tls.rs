use crate::types::{CertStoreSpec, IngressSpec, ServerSpec, TlsTerminationSpec};
use crate::validation::ValidationReport;

pub(crate) fn validate_tls(
    server: &ServerSpec,
    ingresses: &[IngressSpec],
    report: &mut ValidationReport,
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
                    // no-op: already validated in intra_file.
                }
                TlsTerminationSpec::Acme { domains, .. } => {
                    any_acme_listener = true;

                    // ACME requires domains
                    if domains.is_empty() {
                        report.acme_tls_requires_domains(&bind.origin);
                    }
                }
            }
        }
    }

    // If ACME is configured anywhere, server.tls_automation must exist
    if any_acme_listener {
        let Some(tls_automation_cfg) = &server.tls_automation else {
            report.acme_configured_in_ingress_but_server_tls_not_configured(&server.origin);
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
            CertStoreSpec::Filesystem { cert_dir } => {
                if cert_dir.as_os_str().is_empty() {
                    report.server_tls_filesystem_cert_store_must_have_a_cert_directory(
                        &server.origin,
                    );
                }
            }
        }
    }

    // Optional: warn if server.tls_automation exists but no TLS listeners
    if server.tls_automation.is_some() && !any_tls_listener {
        report.warn_server_tls_configured_with_no_tls_listeners(&server.origin);
    }
}
