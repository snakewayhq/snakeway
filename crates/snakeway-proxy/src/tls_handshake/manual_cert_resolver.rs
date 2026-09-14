use arc_swap::ArcSwap;
use pingora_rustls::{ClientHello, ResolvesServerCert, sign};
use snakeway_engine::runtime::RuntimeState;
use std::fmt;
use std::sync::Arc;

/// Serves the manual TLS certificate of one listener from the current `RuntimeState`.
///
/// The certificate is looked up on each handshake, so a reload that loads replaced
/// certificate files takes effect on the next handshake without rebuilding the listener.
pub(crate) struct ManualCertResolver {
    state: Arc<ArcSwap<RuntimeState>>,
    listener_addr: String,
}

impl ManualCertResolver {
    pub(crate) fn new(state: Arc<ArcSwap<RuntimeState>>, listener_addr: String) -> Self {
        Self {
            state,
            listener_addr,
        }
    }

    fn lookup(&self) -> Option<Arc<sign::CertifiedKey>> {
        let runtime = self.state.load();

        let Some(cert) = runtime.manual_certs.get(&self.listener_addr) else {
            tracing::error!(
                "No manual TLS certificate loaded for listener {}",
                self.listener_addr
            );
            return None;
        };

        Some(cert.clone())
    }
}

impl fmt::Debug for ManualCertResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ManualCertResolver")
            .field("listener_addr", &self.listener_addr)
            .finish()
    }
}

impl ResolvesServerCert for ManualCertResolver {
    fn resolve(&self, _client_hello: ClientHello<'_>) -> Option<Arc<sign::CertifiedKey>> {
        self.lookup()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tls_handshake::test_support::{make_runtime_state, make_test_certified_key};
    use std::collections::HashMap;

    const LISTENER_ADDR: &str = "127.0.0.1:8443";

    fn state_with_cert(key: Arc<sign::CertifiedKey>) -> RuntimeState {
        make_runtime_state(None, HashMap::from([(LISTENER_ADDR.to_string(), key)]))
    }

    #[test]
    fn lookup_returns_certificate_for_listener_address() {
        // Arrange
        let key = make_test_certified_key();
        let state = Arc::new(ArcSwap::from_pointee(state_with_cert(key.clone())));
        let resolver = ManualCertResolver::new(state, LISTENER_ADDR.to_string());

        // Act
        let result = resolver.lookup();

        // Assert
        let found = result.expect("the listener certificate must resolve");
        assert!(Arc::ptr_eq(&found, &key));
    }

    #[test]
    fn lookup_returns_none_for_listener_without_certificate() {
        // Arrange
        let key = make_test_certified_key();
        let state = Arc::new(ArcSwap::from_pointee(state_with_cert(key)));
        let resolver = ManualCertResolver::new(state, "127.0.0.1:9443".to_string());

        // Act
        let result = resolver.lookup();

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn lookup_returns_certificate_from_latest_runtime_state() {
        // Arrange
        let first = make_test_certified_key();
        let second = make_test_certified_key();
        let state = Arc::new(ArcSwap::from_pointee(state_with_cert(first.clone())));
        let resolver = ManualCertResolver::new(state.clone(), LISTENER_ADDR.to_string());
        state.store(Arc::new(state_with_cert(second.clone())));

        // Act
        let result = resolver.lookup();

        // Assert
        let found = result.expect("the listener certificate must resolve");
        assert!(Arc::ptr_eq(&found, &second));
        assert!(!Arc::ptr_eq(&found, &first));
    }
}
