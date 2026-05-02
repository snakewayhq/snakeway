use crate::validation::ValidationReport;
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

    //-------------------------------------------------------------------------
    // Validation during transformation
    //-------------------------------------------------------------------------
    #[error("invalid server configuration: {message}")]
    InvalidServerConfig { message: String },

    #[error("invalid admin bind configuration: {message}")]
    InvalidAdminBindConfig { message: String },

    #[error("invalid bind address: {message}")]
    InvalidBindAddress { message: String },

    #[error("invalid bind ip string: {0}")]
    InvalidBindIpString(String),

    #[error("invalid method: {value} (origin: {origin})")]
    InvalidMethod { value: String, origin: String },

    #[error("invalid header name: {value} (origin: {origin})")]
    InvalidHeaderName { value: String, origin: String },

    #[error("invalid upstream: {message}")]
    InvalidUpstream { message: String },

    #[error("validation failed: {validation_report:?}")]
    SemanticValidationFailed { validation_report: ValidationReport },
}

impl ConfigError {
    pub(crate) fn read_file(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::ReadFile {
            path: path.into(),
            source,
        }
    }

    pub(crate) fn parse(path: impl Into<PathBuf>, source: hcl::Error) -> Self {
        Self::Parse {
            path: path.into(),
            source,
        }
    }
}
