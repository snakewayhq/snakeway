use arc_swap::ArcSwap;
use async_trait::async_trait;
use pingora::listeners::TlsAccept;
use pingora::protocols::tls::TlsRef;
use snakeway_engine::DownstreamSni;
use snakeway_engine::runtime::RuntimeState;
use std::any::Any;
use std::sync::Arc;

pub(crate) enum CertMode {
    Manual,
    Acme(Arc<ArcSwap<RuntimeState>>),
}

pub(crate) struct SnakewayTlsAccept {
    cert_mode: CertMode,
}

impl SnakewayTlsAccept {
    pub(crate) fn new(cert_mode: CertMode) -> Self {
        Self { cert_mode }
    }
}

#[async_trait]
impl TlsAccept for SnakewayTlsAccept {
    async fn certificate_callback(&self, ssl: &mut TlsRef) {
        match &self.cert_mode {
            CertMode::Manual => {
                // Do nothing. Cert already configured in settings file.
            }
            CertMode::Acme(state) => {
                // Perform dynamic lookup and install cert based on SNI
                acme_lookup_and_set_cert(state, ssl).await
            }
        }
    }

    async fn handshake_complete_callback(
        &self,
        ssl: &TlsRef,
    ) -> Option<Arc<dyn Any + Send + Sync>> {
        // Extract SNI.
        let hostname = extract_sni(ssl).filter(|s| !s.is_empty())?;
        let hostname = Arc::new(DownstreamSni::new(hostname.clone()));
        Some(hostname)
    }
}

// Perform dynamic lookup and install cert based on SNI
async fn acme_lookup_and_set_cert(state: &Arc<ArcSwap<RuntimeState>>, ssl: &mut TlsRef) {
    // Extract SNI.
    let Some(hostname) = extract_sni(ssl).filter(|s| !s.is_empty()) else {
        return;
    };

    tracing::debug!("TLS handshake: SNI = {}", hostname);

    // Load current runtime state (lock-free).
    let runtime = state.load();

    // If TLS runtime does not exist, nothing to do.
    let Some(tls_runtime) = &runtime.tls else {
        tracing::warn!("TLS requested but runtime has no TLS state");
        return;
    };

    // Load SNI map (lock-free).
    let sni_map = tls_runtime.sni_map.load();

    let Some(cert) = sni_map.get(&hostname) else {
        tracing::warn!("No certificate found for SNI {}", hostname);
        return;
    };

    // Install leaf certificate.
    if let Err(e) = ssl.set_certificate(&cert.leaf) {
        tracing::error!("Failed to install leaf certificate: {}", e);
        return;
    }

    // Install intermediate chain.
    for intermediate in &cert.chain {
        if let Err(e) = ssl.add_chain_cert(intermediate.clone()) {
            tracing::error!("Failed to add intermediate certificate: {}", e);
            return;
        }
    }

    // Install private key.
    if let Err(e) = ssl.set_private_key(&cert.key) {
        tracing::error!("Failed to install private key: {}", e);
        return;
    }

    tracing::debug!("TLS certificate installed successfully for {}", hostname);
}

fn extract_sni(ssl: &TlsRef) -> Option<String> {
    let hostname = match ssl.servername(openssl::ssl::NameType::HOST_NAME) {
        Some(name) => match std::str::from_utf8(name.as_bytes()) {
            Ok(s) => s.to_string(),
            Err(_) => {
                tracing::warn!("Invalid UTF-8 in SNI");
                return None;
            }
        },
        None => {
            tracing::debug!("No SNI provided");
            return None;
        }
    };
    Some(hostname)
}
