use crate::cert_manager::CertStore;
use async_trait::async_trait;
use pingora::listeners::{TlsAccept, TlsAcceptCallbacks};
use pingora::protocols::tls::TlsRef;
use std::sync::Arc;

pub fn build_tls_callbacks(store: Arc<dyn CertStore>) -> TlsAcceptCallbacks {
    Box::new(SnakewayTlsAccept { store })
}

struct SnakewayTlsAccept {
    store: Arc<dyn CertStore>,
}

#[async_trait]
impl TlsAccept for SnakewayTlsAccept {
    async fn certificate_callback(&self, ssl: &mut TlsRef) {
        // Extract SNI.
        let hostname = match ssl.servername(openssl::ssl::NameType::HOST_NAME) {
            Some(name) => match std::str::from_utf8(name.as_bytes()) {
                Ok(s) => s,
                Err(_) => {
                    tracing::warn!("TLS handshake: invalid UTF-8 SNI");
                    return;
                }
            },
            None => {
                tracing::debug!("TLS handshake: no SNI provided");
                return;
            }
        };

        tracing::debug!("TLS handshake: SNI = {}", hostname);

        // Attempt to lookup the cert in the store.
        let cert = match self.store.get(hostname) {
            Some(c) => c,
            None => {
                tracing::warn!("No certificate found for SNI {}", hostname);
                // Handshake will fail naturally if the certificate is not found.
                return;
            }
        };

        // Parse full certificate chain.
        let cert_chain = match openssl::x509::X509::stack_from_pem(&cert.cert_chain_pem) {
            Ok(certs) if !certs.is_empty() => certs,
            _ => {
                tracing::error!("Failed to parse certificate chain for {}", hostname);
                // Handshake will fail naturally if the certificate chain is invalid.
                return;
            }
        };

        // The first cert is the leaf member in the chain.
        let leaf = &cert_chain[0];

        if let Err(e) = ssl.ssl_use_certificate(leaf) {
            tracing::error!("Failed to install leaf certificate: {}", e);
            // Handshake will fail naturally if the leaf cert is invalid.
            return;
        }

        // Add intermediates.
        for intermediate in cert_chain.iter().skip(1) {
            if let Err(e) = ssl.ssl_add_chain_cert(intermediate.clone()) {
                tracing::error!("Failed to add intermediate cert: {}", e);
                // Handshake will fail naturally if any intermediate cert is invalid.
                return;
            }
        }

        // Parse private key.
        let pkey = match openssl::pkey::PKey::private_key_from_pem(&cert.private_key_pem) {
            Ok(k) => k,
            Err(e) => {
                tracing::error!("Failed to parse private key for {}: {}", hostname, e);
                // Handshake will fail naturally if the private key is invalid.
                return;
            }
        };

        // Attempt to install the private key.
        if let Err(e) = ssl.ssl_use_private_key(&pkey) {
            tracing::error!("Failed to install private key: {}", e);
            // Handshake will fail naturally if the private key cannot be installed.
            return;
        }

        // Success.
        tracing::debug!("TLS certificate installed successfully for {}", hostname);
    }
}
