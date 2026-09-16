use std::ffi::OsString;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    //-------------------------------------------------------------------------
    // IO / Discovery
    //-------------------------------------------------------------------------
    #[error("failed to read config file {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "config file {path} is {size} bytes, larger than the maximum supported size of {max} bytes (4 GiB). Split the configuration into multiple files under the config directory."
    )]
    FileTooLarge {
        path: PathBuf,
        size: usize,
        max: u32,
    },

    #[error("glob pattern error: {pattern}: {source}")]
    Glob {
        pattern: String,
        #[source]
        source: glob::PatternError,
    },

    #[error("failed to resolve glob pattern {pattern} relative to {root}: {os_string:?}")]
    ResolveGlob {
        root: String,
        pattern: String,
        os_string: OsString,
    },

    //-------------------------------------------------------------------------
    // Validation during transformation
    //-------------------------------------------------------------------------
    #[error("invalid bind ip string: {0}")]
    InvalidBindIpString(String),

    #[error("validation failed: {report:?}")]
    SemanticValidationFailed {
        report: confval::diagnostic::Report,
        sources: confval::source::SourceMap,
    },

    //-------------------------------------------------------------------------
    // Lowering
    //-------------------------------------------------------------------------
    #[error("server lowering returned None without reporting an error")]
    ServerLoweringReturnedNone,

    #[error("config lowering returned None without reporting an error")]
    ConfigLoweringReturnedNone,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowering_variants_have_distinct_messages() {
        // Arrange
        let server = ConfigError::ServerLoweringReturnedNone;
        let config = ConfigError::ConfigLoweringReturnedNone;

        // Act
        let messages = (server.to_string(), config.to_string());

        // Assert
        assert_ne!(
            messages.0, messages.1,
            "the two lowering stages must be distinguishable in diagnostics"
        );
        assert_eq!(
            messages.1,
            "config lowering returned None without reporting an error"
        );
    }

    #[test]
    fn file_too_large_message_states_size_limit_and_remedy() {
        // Arrange
        let error = ConfigError::FileTooLarge {
            path: PathBuf::from("ingress.d/api.hcl"),
            size: 5_000_000_000,
            max: u32::MAX,
        };

        // Act
        let message = error.to_string();

        // Assert
        assert_eq!(
            message,
            "config file ingress.d/api.hcl is 5000000000 bytes, larger than the \
             maximum supported size of 4294967295 bytes (4 GiB). Split the \
             configuration into multiple files under the config directory."
        );
    }
}
