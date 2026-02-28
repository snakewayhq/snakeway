use crate::conf::types::{AcmeServerSpec, CertStoreSpec, ServerSpec};
use crate::conf::validation::report::ValidationReport;
use crate::conf::validation::validator::{
    SERVER_THREADS, SERVER_TLS_RENEW_WITHIN_DAYS, validate_cert_pem, validate_range,
};
use nix::NixPath;

/// Validate top-level config version.
///
/// Fail-fast: invalid versions invalidate the entire config model.
pub fn validate_version(server_spec: &ServerSpec, report: &mut ValidationReport) -> bool {
    if server_spec.version != 1 {
        report.invalid_config_version(&server_spec.version, &server_spec.origin);
        return false;
    }
    true
}

/// Validate the server config.
///
/// Version validation fails fast, because it invalidates the entire config model.
pub fn validate_server(server_spec: &ServerSpec, report: &mut ValidationReport) {
    if let Some(pid_file) = server_spec.pid_file.clone() {
        let Some(parent) = pid_file.parent() else {
            return;
        };

        if !parent.exists() {
            report.pid_file_parent_dir_does_not_exist(pid_file.display(), &server_spec.origin);
        } else if !parent.is_dir() {
            report.pid_file_parent_not_a_dir(pid_file.display(), &server_spec.origin);
        }
    }

    if let Some(ca_file) = &server_spec.ca_file {
        if let Err(e) = validate_cert_pem(ca_file) {
            report.server_ca_file_invalid(ca_file, &e, &server_spec.origin);
        }
    }

    if let Some(t) = server_spec.threads
        && (t == 0 || t > 1024)
    {
        validate_range(t, &SERVER_THREADS, report, &server_spec.origin);
    }

    if let Some(tls_automation_cfg) = &server_spec.tls_automation {
        // ACME.
        let AcmeServerSpec {
            directory_url,
            contact_email,
            ca_file,
            data_dir,
        } = &tls_automation_cfg.acme;

        if directory_url.is_empty() {
            report.server_tls_acme_directory_url_cannot_be_empty(&server_spec.origin);
        } else if !directory_url.starts_with("https://") {
            report.server_tls_acme_directory_url_must_be_https(&server_spec.origin);
        }

        if contact_email.is_empty() {
            report.server_tls_acme_contact_email_cannot_be_empty(&server_spec.origin);
        }

        if let Some(ca_file) = &ca_file {
            if let Err(e) = validate_cert_pem(ca_file) {
                report.server_tls_acme_ca_file_invalid(ca_file, &e, &server_spec.origin);
            }
        }

        if !data_dir.is_dir() {
            report.server_tls_acme_data_dir_is_invalid(data_dir, &server_spec.origin);
        }

        // Renewal period.
        validate_range(
            tls_automation_cfg.renew_within_days,
            &SERVER_TLS_RENEW_WITHIN_DAYS,
            report,
            &server_spec.origin,
        );

        // Cert store.
        match &tls_automation_cfg.cert_store {
            CertStoreSpec::Filesystem { cert_dir } => {
                if cert_dir.is_empty() {
                    report.server_tls_filesystem_cert_store_must_have_a_cert_directory(
                        &server_spec.origin,
                    );
                }
            }
            CertStoreSpec::Memory => {
                // Nothing to do here.
            }
        }
    }
}
