//! TLS upstream starters.

use crate::constants::{HTTP_UPSTREAM_RESPONSE, TEST_HOST};
use std::thread;

/// Build a TLS acceptor for a test upstream from PEM text.
///
/// The TLS config uses the ring provider directly, because a test upstream starts before
/// the proxy installs a process-level rustls provider.
pub(super) fn test_upstream_tls_acceptor(
    cert_pem: &str,
    key_pem: &str,
) -> tokio_rustls::TlsAcceptor {
    use rustls::pki_types::pem::PemObject;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    use std::sync::Arc;

    let certs = CertificateDer::pem_slice_iter(cert_pem.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .expect("invalid upstream certificate PEM");
    let key = PrivateKeyDer::from_pem_slice(key_pem.as_bytes()).expect("invalid upstream key PEM");
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("unsupported TLS protocol versions")
    .with_no_client_auth()
    .with_single_cert(certs, key)
    .expect("invalid upstream certificate and key");
    tokio_rustls::TlsAcceptor::from(Arc::new(config))
}

/// Answer one TLS connection with the plain upstream response.
async fn respond_over_tls<S>(acceptor: tokio_rustls::TlsAcceptor, stream: S)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let Ok(mut tls) = acceptor.accept(stream).await else {
        return;
    };
    let mut request = [0u8; 4096];
    let _ = tls.read(&mut request).await;
    let _ = tls.write_all(HTTP_UPSTREAM_RESPONSE).await;
    let _ = tls.shutdown().await;
}

/// An HTTPS upstream that presents the given PEM certificate and private key.
pub fn start_https_upstream(cert_pem: &str, key_pem: &str) -> u16 {
    let acceptor = test_upstream_tls_acceptor(cert_pem, key_pem);
    let (listener, port) = super::plain::bind_ephemeral();
    listener
        .set_nonblocking(true)
        .expect("failed to set nonblocking");

    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let listener =
                tokio::net::TcpListener::from_std(listener).expect("failed to adopt listener");
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                tokio::spawn(respond_over_tls(acceptor.clone(), stream));
            }
        });
    });

    port
}

/// An HTTPS upstream on a Unix domain socket. The socket is bound before this returns.
pub fn start_unix_https_upstream(path: &std::path::Path, cert_pem: &str, key_pem: &str) {
    let acceptor = test_upstream_tls_acceptor(cert_pem, key_pem);
    let listener =
        std::os::unix::net::UnixListener::bind(path).expect("failed to bind unix socket upstream");
    listener
        .set_nonblocking(true)
        .expect("failed to set nonblocking");

    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let listener =
                tokio::net::UnixListener::from_std(listener).expect("failed to adopt listener");
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                tokio::spawn(respond_over_tls(acceptor.clone(), stream));
            }
        });
    });
}

/// A self-signed certificate for an upstream, so no system trust store holds it.
/// Returns the cert and key PEM strings.
pub fn generate_upstream_identity() -> (String, String) {
    let generated = rcgen::generate_simple_self_signed(vec![TEST_HOST.into()])
        .expect("failed to generate upstream certificate");
    (generated.cert.pem(), generated.signing_key.serialize_pem())
}
