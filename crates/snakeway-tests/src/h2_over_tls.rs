use crate::constants::TEST_HOST;
use bytes::Bytes;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

/// Sends `req` over an HTTP/2-over-TLS connection to `addr` and returns the
/// response status, or an error string when the request is refused, reset, or
/// times out before a response arrives.
pub fn send_h2_request(
    addr: &str,
    req: http::Request<()>,
    end_stream: bool,
) -> Result<http::StatusCode, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    rt.block_on(async move {
        let send = connect_h2_over_tls(addr).await;
        let mut send = send
            .ready()
            .await
            .map_err(|e| format!("connection not ready: {e}"))?;

        let (resp, _stream) = send
            .send_request(req, end_stream)
            .map_err(|e| e.to_string())?;

        match tokio::time::timeout(Duration::from_secs(10), resp).await {
            Ok(Ok(response)) => Ok(response.status()),
            Ok(Err(e)) => Err(e.to_string()),
            Err(_) => Err("response timed out".to_string()),
        }
    })
}

/// Accepts any server certificate.
/// Test-only: the harness listener presents a self-signed certificate.
#[derive(Debug)]
pub struct NoCertVerify;

impl ServerCertVerifier for NoCertVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
        ]
    }
}

async fn connect_h2_over_tls(addr: &str) -> h2::client::SendRequest<Bytes> {
    let provider = Arc::new(tokio_rustls::rustls::crypto::aws_lc_rs::default_provider());
    let mut config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("tls protocol versions")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoCertVerify))
        .with_no_client_auth();
    config.alpn_protocols = vec![b"h2".to_vec()];

    let connector = TlsConnector::from(Arc::new(config));
    let tcp = TcpStream::connect(addr).await.expect("tcp connect");
    let server_name = ServerName::try_from(TEST_HOST).expect("valid server name");
    let tls = connector
        .connect(server_name, tcp)
        .await
        .expect("tls handshake");

    let (send, connection) = h2::client::handshake(tls).await.expect("h2 handshake");
    tokio::spawn(async move {
        let _ = connection.await;
    });
    send
}
