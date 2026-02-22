use crate::conf::types::{CertStoreSpec, ServerSpec};
use crate::conf::validation::report::ValidationReport;
use crate::conf::validation::validator::{
    SERVER_THREADS, SERVER_TLS_RENEW_WITHIN_DAYS, validate_range,
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

    if let Some(ca_file) = server_spec.ca_file.clone() {
        if !std::path::Path::new(&ca_file).exists() {
            report.root_ca_file_does_not_exist(&ca_file, &server_spec.origin);
        }
        if !std::path::Path::new(&ca_file).is_file() {
            report.root_ca_file_not_a_file(&ca_file, &server_spec.origin);
        }
    }

    if let Some(t) = server_spec.threads
        && (t == 0 || t > 1024)
    {
        validate_range(t, &SERVER_THREADS, report, &server_spec.origin);
    }

    if let Some(tls) = &server_spec.tls {
        if let Some(renew_within_days) = &tls.renew_within_days {
            validate_range(
                *renew_within_days,
                &SERVER_TLS_RENEW_WITHIN_DAYS,
                report,
                &server_spec.origin,
            );
        } else {
            report.server_tls_renew_within_days_must_be_set(&server_spec.origin);
        }

        match &tls.cert_store {
            CertStoreSpec::Filesystem(cert_dir) => {
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
