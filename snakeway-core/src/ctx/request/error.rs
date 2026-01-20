use pingora::{BError, Custom, Error as PingoraError};
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum RequestRejectError {
    #[error("invalid request path")]
    InvalidPath,

    #[error("request normalization failed")]
    NormalizationFailure,
}

impl RequestRejectError {
    pub(crate) fn as_pingora_error(&self) -> BError {
        match self {
            Self::InvalidPath => PingoraError::new(Custom("invalid_request_path")),
            Self::NormalizationFailure => PingoraError::new(Custom("request_normalization_failed")),
        }
    }
}
