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
        if !pid_file.exists() {
            ctx.push(ConfigError::InvalidPidFile {
                pid_file: pid_file.clone(),
                reason: "file does not exist".to_string(),
            });
        }

        if !pid_file.is_file() {
            ctx.push(ConfigError::InvalidPidFile {
                pid_file: pid_file.clone(),
                reason: "it exists, but is not a file".to_string(),
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
