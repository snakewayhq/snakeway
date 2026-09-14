use arc_swap::ArcSwap;
use async_trait::async_trait;
use pingora::listeners::TlsAccept;
use pingora::protocols::tls::TlsRef;
use pingora_rustls::{ClientHello, ResolvesServerCert, sign};
use snakeway_engine::DownstreamSni;
use snakeway_engine::runtime::RuntimeState;
use std::any::Any;
use std::fmt;
use std::sync::Arc;

/// Selects a TLS certificate at handshake time based on the client SNI.
///
/// When ACME is active, certificates are obtained and stored at runtime.
/// This resolver reads the current SNI map from `RuntimeState` and returns
/// the matching `CertifiedKey`.
pub(crate) struct SnakewayCertResolver {
    state: Arc<ArcSwap<RuntimeState>>,
}

impl SnakewayCertResolver {
    pub(crate) fn new(state: Arc<ArcSwap<RuntimeState>>) -> Self {
        Self { state }
    }
}

impl fmt::Debug for SnakewayCertResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SnakewayCertResolver").finish()
    }
}

impl ResolvesServerCert for SnakewayCertResolver {
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<Arc<sign::CertifiedKey>> {
        let hostname = client_hello.server_name()?;

        tracing::debug!("TLS handshake: SNI = {}", hostname);

        let runtime = self.state.load();

        let tls_runtime = match &runtime.tls {
            Some(tls) => tls,
            None => {
                tracing::warn!("TLS requested but runtime has no TLS state");
                return None;
            }
        };

        let sni_map = tls_runtime.sni_map.load();

        let cert = match sni_map.get(hostname) {
            Some(c) => c,
            None => {
                tracing::warn!("No certificate found for SNI {}", hostname);
                return None;
            }
        };

        Some(cert.certified_key().clone())
    }
}

/// Extracts the SNI hostname after the TLS handshake completes and stores
/// it as `DownstreamSni` for the proxy layer.
pub(crate) struct SnakewayTlsAccept;

#[async_trait]
impl TlsAccept for SnakewayTlsAccept {
    async fn handshake_complete_callback(
        &self,
        ssl: &TlsRef,
    ) -> Option<Arc<dyn Any + Send + Sync>> {
        let hostname = ssl.server_name().filter(|s| !s.is_empty())?;
        let sni = Arc::new(DownstreamSni::new(hostname.to_string()));
        Some(sni)
    }
}
