use pingora::{BError, Custom, Error as PingoraError};
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum RequestRejectError {
    #[error("invalid request path")]
    InvalidPath,

    #[error("request normalization failed")]
    NormalizationFailure,

    #[error("invalid query string")]
    InvalidQueryString,

    #[error("invalid headers")]
    InvalidHeaders,

    #[error("invalid method")]
    InvalidMethod,
}

impl RequestRejectError {
    pub(crate) fn as_pingora_error(&self) -> BError {
        match self {
            Self::InvalidPath => PingoraError::new(Custom("invalid request path")),
            Self::NormalizationFailure => PingoraError::new(Custom("request normalization failed")),
            Self::InvalidQueryString => PingoraError::new(Custom("invalid query string")),
            Self::InvalidHeaders => PingoraError::new(Custom("invalid headers")),
            Self::InvalidMethod => PingoraError::new(Custom("invalid method")),
        }
    }
}
