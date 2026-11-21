//! The error type shared by the ACME order reconciler and its helpers.

use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ReconcilerError {
    #[error("acme client not available: {0}")]
    CannotGetAcmeClient(String),

    #[error("acme error: {0}")]
    Acme(String),

    #[error("order store error: {0}")]
    OrderStore(String),

    #[error("cert store error: {0}")]
    CertStore(String),

    #[error("no pending authorization found")]
    NoPendingAuthorization,

    #[error("no http-01 challenge found")]
    NoHttp01Challenge,

    #[error("unexpected order status: {0:?}")]
    UnexpectedOrderStatus(instant_acme::OrderStatus),
}
