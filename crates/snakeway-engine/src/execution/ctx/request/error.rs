use pingora::{BError, Custom, Error as PingoraError};
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum RequestRejectError {
    #[error("invalid request path")]
    InvalidPath,

    #[error("invalid query string")]
    InvalidQueryString,

    #[error("invalid headers")]
    InvalidHeaders,

    #[error("invalid method")]
    InvalidMethod,

    #[error("host and SNI must match if SNI is present")]
    HostSniMismatch,

    #[error("invalid host header")]
    InvalidHostHeader,
}

impl RequestRejectError {
    pub fn as_pingora_error(&self) -> BError {
        match self {
            Self::InvalidPath => PingoraError::new(Custom("invalid request path")),
            Self::InvalidQueryString => PingoraError::new(Custom("invalid query string")),
            Self::InvalidHeaders => PingoraError::new(Custom("invalid headers")),
            Self::InvalidMethod => PingoraError::new(Custom("invalid method")),
            Self::HostSniMismatch => {
                PingoraError::new(Custom("host and SNI must match if SNI is present"))
            }
            Self::InvalidHostHeader => PingoraError::new(Custom("invalid host header")),
        }
    }
}
