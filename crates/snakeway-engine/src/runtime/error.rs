use snakeway_conf::validation::ConfigError;

#[derive(Debug, thiserror::Error)]
pub enum ReloadError {
    #[error("failed to load configuration: {0}")]
    Load(#[from] ConfigError),

    #[error("failed to build runtime state: {0:#}")]
    Build(#[from] anyhow::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_error_message_includes_the_cause() {
        // Arrange
        let error = ReloadError::Build(
            anyhow::anyhow!("connection refused").context("failed to resolve upstream hostname"),
        );

        // Act
        let message = error.to_string();

        // Assert
        assert!(
            message.contains("failed to resolve upstream hostname"),
            "{message}"
        );
        assert!(message.contains("connection refused"), "{message}");
    }
}
