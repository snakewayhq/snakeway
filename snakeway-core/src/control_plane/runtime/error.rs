use snakeway_conf::validation::{ConfigError, ValidationReport};

#[derive(Debug, thiserror::Error)]
pub(crate) enum ReloadError {
    #[error("failed to load configuration")]
    Load(#[from] ConfigError),

    #[error("configuration validation failed")]
    InvalidConfig { report: ValidationReport },

    #[error("failed to build runtime state")]
    Build(#[from] anyhow::Error),
}
