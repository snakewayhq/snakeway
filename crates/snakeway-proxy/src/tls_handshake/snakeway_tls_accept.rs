use async_trait::async_trait;
use pingora::listeners::TlsAccept;
use pingora::protocols::tls::TlsRef;
use snakeway_engine::DownstreamSni;
use std::any::Any;
use std::sync::Arc;

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
