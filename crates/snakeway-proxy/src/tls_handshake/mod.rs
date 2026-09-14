mod cert_resolver;
mod snakeway_tls_accept;

use pingora::listeners::TlsAcceptCallbacks;
pub(crate) use cert_resolver::SnakewayCertResolver;

pub(crate) fn build_tls_callbacks() -> TlsAcceptCallbacks {
    Box::new(snakeway_tls_accept::SnakewayTlsAccept)
}
