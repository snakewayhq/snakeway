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

    #[error("missing method")]
    MissingMethod,

    #[error("request not normalized")]
    NotNormalized,

    #[error("host and SNI must match if SNI is present")]
    HostSniMismatch,

    #[error("invalid host header")]
    InvalidHostHeader,
}

impl RequestRejectError {
    pub(crate) fn as_pingora_error(&self) -> BError {
        match self {
            Self::InvalidPath => PingoraError::new(Custom("invalid request path")),
            Self::NormalizationFailure => PingoraError::new(Custom("request normalization failed")),
            Self::InvalidQueryString => PingoraError::new(Custom("invalid query string")),
            Self::InvalidHeaders => PingoraError::new(Custom("invalid headers")),
            Self::InvalidMethod => PingoraError::new(Custom("invalid method")),
            Self::MissingMethod => PingoraError::new(Custom("missing method")),
            Self::NotNormalized => PingoraError::new(Custom("request not normalized")),
            Self::HostSniMismatch => {
                PingoraError::new(Custom("host and SNI must match if SNI is present"))
            }
            Self::InvalidHostHeader => PingoraError::new(Custom("invalid host header")),
        }
    }
}
