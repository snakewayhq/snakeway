use crate::conf::types::ServerConfig;
use crate::conf::validation::error::ConfigError;
use crate::conf::validation::validation_ctx::ValidationCtx;

/// Validate top-level config version.
///
/// Fail-fast: invalid versions invalidate the entire config model.
pub fn validate_version(version: u32) -> Result<(), ConfigError> {
    if version == 1 {
        Ok(())
    } else {
        Err(ConfigError::InvalidVersion { version })
    }
}

/// Validate the server config.
///
/// Version validation fails fast, because it invalidates the entire config model.
pub fn validate_server(cfg: &ServerConfig, ctx: &mut ValidationCtx) {
    if let Some(pid_file) = cfg.pid_file.clone() {
        let Some(parent) = pid_file.parent() else {
            return;
        };

        if !parent.exists() {
            ctx.push(ConfigError::InvalidPidFile {
                pid_file: pid_file.clone(),
                reason: "parent directory does not exist".to_string(),
            });
        } else if !parent.is_dir() {
            ctx.push(ConfigError::InvalidPidFile {
                pid_file: pid_file.clone(),
                reason: "parent path exists but is not a directory".to_string(),
            });
        }
    }

    if let Some(t) = cfg.threads
        && (t == 0 || t > 1024)
    {
        ctx.push(ConfigError::InvalidThreads {
            threads: cfg.threads.unwrap(),
            reason: "must be between 1 and 1024".to_string(),
        });
    }
}
