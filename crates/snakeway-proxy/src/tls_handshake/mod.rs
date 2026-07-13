mod snakeway_tls_accept;

use pingora::listeners::TlsAcceptCallbacks;
pub(crate) use snakeway_tls_accept::{CertMode, SnakewayTlsAccept};

pub(crate) fn build_tls_callbacks(mode: CertMode) -> TlsAcceptCallbacks {
    Box::new(SnakewayTlsAccept::new(mode))
}
