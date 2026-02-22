use thiserror::Error;

#[derive(Debug, Error)]
pub enum CertManagerError {
    #[error("certificate not found: {0}")]
    NotFound(String),

    #[error("failed to parse certificate chain: {0}")]
    InvalidChain(String),

    #[error("certificate chain is empty")]
    EmptyChain,

    #[error("failed to parse private key: {0}")]
    InvalidPrivateKey(String),

    #[error("certificate and private key do not match")]
    KeyMismatch,
}
