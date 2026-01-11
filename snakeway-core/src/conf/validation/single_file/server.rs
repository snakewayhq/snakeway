use crate::conf::types::ServerSpec;
use crate::conf::validation::report::ValidationReport;
use crate::conf::validation::validator::range::{SERVER_THREADS, validate_range};

/// Validate top-level config version.
///
/// Fail-fast: invalid versions invalidate the entire config model.
pub fn validate_version(server: &ServerSpec, report: &mut ValidationReport) -> bool {
    if server.version != 1 {
        report.invalid_config_version(&server.version, &server.origin);
        return false;
    }
    true
}

/// Validate the server config.
///
/// Version validation fails fast, because it invalidates the entire config model.
pub fn validate_server(cfg: &ServerSpec, report: &mut ValidationReport) {
    if let Some(pid_file) = cfg.pid_file.clone() {
        let Some(parent) = pid_file.parent() else {
            return;
        };

        if !parent.exists() {
            report.pid_file_parent_dir_does_not_exist(pid_file.display(), &cfg.origin);
        } else if !parent.is_dir() {
            report.pid_file_parent_not_a_dir(pid_file.display(), &cfg.origin);
        }
    }

    if let Some(ca_file) = cfg.ca_file.clone() {
        if !std::path::Path::new(&ca_file).exists() {
            report.root_ca_file_does_not_exist(&ca_file, &cfg.origin);
        }
        if !std::path::Path::new(&ca_file).is_file() {
            report.root_ca_file_not_a_file(&ca_file, &cfg.origin);
        }
    }

    if let Some(t) = cfg.threads
        && (t == 0 || t > 1024)
    {
        validate_range(t, &SERVER_THREADS, report, &cfg.origin);
    }
}
