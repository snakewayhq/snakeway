use crate::conf::types::ExposeServerConfig;
use crate::conf::validation::report::ValidationReport;

/// Validate top-level config version.
///
/// Fail-fast: invalid versions invalidate the entire config model.
pub fn validate_version(server: &ExposeServerConfig, report: &mut ValidationReport) -> bool {
    if server.version != 1 {
        report.error(
            format!("invalid config version {}", server.version),
            &server.origin,
            None,
        );
        return false;
    }
    true
}

/// Validate the server config.
///
/// Version validation fails fast, because it invalidates the entire config model.
pub fn validate_server(cfg: &ExposeServerConfig, report: &mut ValidationReport) {
    if let Some(pid_file) = cfg.pid_file.clone() {
        let Some(parent) = pid_file.parent() else {
            return;
        };

        if !parent.exists() {
            report.error(
                format!(
                    "invalid pid file - parent directory does not exist: {}",
                    pid_file.display()
                ),
                &cfg.origin,
                None,
            );
        } else if !parent.is_dir() {
            report.error(
                format!(
                    "invalid pid file - parent path exists but is not a directory: {}",
                    pid_file.display()
                ),
                &cfg.origin,
                None,
            );
        }
    }

    if let Some(ca_file) = cfg.ca_file.clone() {
        if !std::path::Path::new(&ca_file).exists() {
            report.error(
                format!("invalid root CA file - file does not exist: {}", ca_file),
                &cfg.origin,
                None,
            );
        }
        if !std::path::Path::new(&ca_file).is_file() {
            report.error(
                format!(
                    "invalid root CA file - path exists but is not a file: {}",
                    ca_file
                ),
                &cfg.origin,
                None,
            );
        }
    }

    if let Some(t) = cfg.threads
        && (t == 0 || t > 1024)
    {
        report.error(
            format!("invalid threads - must be between 1 and 1024: {}", t),
            &cfg.origin,
            None,
        );
    }
}
