use crate::conf::validation::error::ConfigError;

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
