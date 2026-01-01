use crate::conf::validation::validation_ctx::ValidationErrors;
use miette::Diagnostic;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
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

    //-------------------------------------------------------------------------
    // Top-level
    //-------------------------------------------------------------------------
    #[error("invalid version '{version}'")]
    InvalidVersion { version: u32 },

    #[error("invalid pid file path '{pid_file}': {reason}")]
    InvalidPidFile { pid_file: PathBuf, reason: String },

    #[error("config validation failed  {validation_errors:?}")]
    Validation { validation_errors: ValidationErrors },

    //-------------------------------------------------------------------------
    // Parsing
    //-------------------------------------------------------------------------
    #[error("invalid configuration file: {path}\n\n{source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    //-------------------------------------------------------------------------
    // Merge / Structure
    //-------------------------------------------------------------------------
    #[error("duplicate service definition: {name}")]
    DuplicateService { name: String },

    #[error("duplicate route for path '{path}'")]
    DuplicateRoute { path: String },

    //-------------------------------------------------------------------------
    // Routes
    //-------------------------------------------------------------------------
    #[error("invalid route '{path}'")]
    InvalidRoute { path: String },

    #[error("invalid route path '{path}': {reason}")]
    InvalidRoutePath { path: String, reason: String },

    #[error("static route directory does not exist or is not a directory: {path} ({reason})")]
    InvalidStaticDir { path: PathBuf, reason: String },

    #[error("route '{path}' is declared as type=static and cannot enable WebSockets")]
    WebSocketNotAllowedOnStaticRoute { path: String },

    #[error("service route '{path}' is missing required 'service' field")]
    MissingServiceForServiceRoute { path: String },

    #[error("route '{path}' is declared as type=static but is missing required field: dir")]
    MissingDirForStaticRoute { path: String },

    #[error("route '{path}' is declared as type=static and must not define 'service'")]
    ServiceNotAllowedOnStaticRoute { path: String },

    #[error("service route '{path}' must not define 'dir'")]
    DirNotAllowedOnServiceRoute { path: String },

    //-------------------------------------------------------------------------
    // Listeners
    //-------------------------------------------------------------------------
    #[error("invalid listener socket address '{addr}'")]
    InvalidListenerAddr { addr: String },

    #[error("duplicate listener address '{addr}'")]
    DuplicateListenerAddr { addr: String },

    #[error("cert file does not exist: {path}")]
    MissingCertFile { path: String },

    #[error("key file does not exist: {path}")]
    MissingKeyFile { path: String },

    #[error("HTTP/2 requires TLS to be configured on the listener")]
    Http2RequiresTls,

    #[error("HTTP/2 is not supported on admin listeners")]
    AdminListenerHttp2NotSupported,

    #[error("admin listener must use TLS")]
    AdminListenerMissingTls,

    #[error("only one admin listener may be defined")]
    MultipleAdminListeners,

    //-------------------------------------------------------------------------
    // Services
    //-------------------------------------------------------------------------
    #[error("route '{path}' references unknown service '{service}'")]
    UnknownService { path: String, service: String },

    #[error("service '{service}' has no upstreams defined")]
    EmptyService { service: String },

    #[error("invalid load balancing strategy '{strategy}' for service '{service}'")]
    InvalidLoadBalancingStrategy { service: String, strategy: String },

    #[error("invalid circuit breaker config for service '{service}': {reason}")]
    InvalidCircuitBreaker { service: String, reason: String },

    #[error("invalid upstream '{upstream}' for service '{service}': {reason}")]
    InvalidUpstream {
        service: String,
        upstream: String,
        reason: String,
    },
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
