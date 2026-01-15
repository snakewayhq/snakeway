use std::net::SocketAddr;

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("failed to resolve hostname '{0}'")]
    DnsFailed(String),

    #[error("hostname '{0}' resolved to no addresses")]
    NoAddresses(String),

    #[error("bind failed for {0}")]
    BindFailed(SocketAddr),

    #[error("address family not supported: {0}")]
    UnsupportedAddress(SocketAddr),

    #[error("io error during resolution: {0}")]
    Io(#[from] std::io::Error),
}
