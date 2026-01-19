use thiserror::Error;

#[derive(Debug, Error)]
pub enum RequestRejectError {
    #[error("invalid request path")]
    InvalidPath,

    #[error("request normalization failed")]
    NormalizationFailure,
}
