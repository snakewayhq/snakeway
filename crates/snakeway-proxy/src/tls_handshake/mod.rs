mod snakeway_tls_accept;

use pingora::listeners::TlsAcceptCallbacks;
pub(crate) use snakeway_tls_accept::{SnakewayCertResolver, SnakewayTlsAccept};

pub(crate) fn build_tls_callbacks() -> TlsAcceptCallbacks {
    Box::new(SnakewayTlsAccept)
}
