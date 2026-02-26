use crate::conf::types::{CertStoreSpec, CertificateSpec, IngressSpec, ServerSpec};
use crate::conf::validation::ValidationReport;

pub fn validate_tls(server: &ServerSpec, ingresses: &[IngressSpec], report: &mut ValidationReport) {
    let mut any_tls_listener = false;
    let mut any_acme_listener = false;

    for ingress in ingresses {
        if let Some(bind) = &ingress.bind {
            if let Some(certificate_spec) = &bind.tls {
                any_tls_listener = true;

                match certificate_spec {
                    CertificateSpec::Static { .. } => {
                        // no-op: already validated in single_file.
                    }
                    CertificateSpec::Acme { domains, .. } => {
                        any_acme_listener = true;

                        // ACME requires domains
                        if domains.is_empty() {
                            report.acme_tls_requires_domains(&bind.origin);
                        }
                    }
                }
            }
        }
    }

    // If ACME is configured anywhere, server.tls must exist
    if any_acme_listener {
        let Some(server_tls) = &server.certificates else {
            report.acme_configured_in_ingress_but_server_tls_not_configured(&server.origin);
            return;
        };

        match &server_tls.cert_store {
            CertStoreSpec::Memory => {
                report.acme_requires_durable_cert_store(&server.origin);
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

    // Optional: warn if server.tls exists but no TLS listeners
    if server.certificates.is_some() && !any_tls_listener {
        report.warn_server_tls_configured_with_no_tls_listeners(&server.origin);
    }
}
