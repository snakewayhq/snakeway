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

    #[error("glob pattern error: {pattern}: {source}")]
    Glob {
        pattern: String,
        #[source]
        source: glob::PatternError,
    },

    #[error("message")]
    Custom { message: String },

    //-------------------------------------------------------------------------
    // Parsing
    //-------------------------------------------------------------------------
    #[error("invalid configuration file: {path}\n\n{source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: hcl::Error,
    },
}

impl ConfigError {
    pub fn read_file(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::ReadFile {
            path: path.into(),
            source,
        }
    }

    pub fn parse(path: impl Into<PathBuf>, source: hcl::Error) -> Self {
        Self::Parse {
            path: path.into(),
            source,
        }
    }
}
