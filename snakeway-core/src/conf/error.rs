use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    // IO / Discovery
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

    // Parsing
    #[error("failed to parse TOML in {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    // Merge / Structure
    #[error("duplicate service definition: {name}")]
    DuplicateService { name: String },

    #[error("duplicate route for path {path}")]
    DuplicateRoute { path: String },

    // Validation
    #[error("route '{route}' references unknown service '{service}'")]
    UnknownService { route: String, service: String },

    #[error("service '{service}' has no upstreams defined")]
    EmptyService { service: String },

    #[error("invalid load balancing strategy '{strategy}' for service '{service}'")]
    InvalidStrategy { service: String, strategy: String },

    // Top-level
    #[error("invalid configuration")]
    InvalidConfig,

    #[error("invalid route '{path}'")]
    InvalidRoute { path: String },
}

impl ConfigError {
    pub fn read_file(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::ReadFile {
            path: path.into(),
            source,
        }
    }

    pub fn parse(path: impl Into<PathBuf>, source: toml::de::Error) -> Self {
        Self::Parse {
            path: path.into(),
            source,
        }
    }
}
