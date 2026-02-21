mod downstream_sni;
mod snakeway_tls_accept;

pub use downstream_sni::DownstreamSni;
use pingora::listeners::TlsAcceptCallbacks;
pub use snakeway_tls_accept::{CertMode, SnakewayTlsAccept};

pub fn build_tls_callbacks(mode: CertMode) -> TlsAcceptCallbacks {
    Box::new(SnakewayTlsAccept::new(mode))
}
