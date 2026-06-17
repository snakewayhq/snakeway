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
}
