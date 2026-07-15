use snakeway_conf::validation::ConfigError;

#[derive(Debug, thiserror::Error)]
pub enum ReloadError {
    #[error("failed to load configuration")]
    Load(#[from] ConfigError),

    #[error("failed to build runtime state")]
    Build(#[from] anyhow::Error),
}
