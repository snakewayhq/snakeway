use arc_swap::ArcSwap;
use pingora_rustls::{ClientHello, ResolvesServerCert, sign};
use snakeway_engine::runtime::RuntimeState;
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

    fn lookup(&self, hostname: &str) -> Option<Arc<sign::CertifiedKey>> {
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

        Some(cert.clone())
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
        self.lookup(hostname)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snakeway_acme::SniRegistry;
    use snakeway_engine::runtime::TlsRuntime;
    use std::collections::HashMap;

    fn make_test_certified_key() -> Arc<sign::CertifiedKey> {
        pingora_rustls::install_default_crypto_provider();
        let cert = rcgen::generate_simple_self_signed(vec!["test.example".into()])
            .expect("failed to generate cert");
        let cert_pem = cert.cert.pem();
        let key_pem = cert.signing_key.serialize_pem();

        let certs: Vec<_> =
            rustls_pemfile::certs(&mut std::io::Cursor::new(cert_pem.as_bytes()))
                .collect::<Result<Vec<_>, _>>()
                .expect("failed to parse cert PEM");

        let key = rustls_pemfile::private_key(&mut std::io::Cursor::new(key_pem.as_bytes()))
            .expect("failed to parse key PEM")
            .expect("no key found");

        let provider =
            pingora_rustls::CryptoProvider::get_default().expect("provider not installed");

        Arc::new(
            sign::CertifiedKey::from_der(certs, key, provider).expect("failed to build CertifiedKey"),
        )
    }

    fn make_resolver_with_sni(
        entries: Vec<(&str, Arc<sign::CertifiedKey>)>,
    ) -> SnakewayCertResolver {
        let mut map = HashMap::new();
        for (domain, key) in entries {
            map.insert(domain.to_string(), key);
        }
        let registry = Arc::new(SniRegistry::new(map));
        let state = RuntimeState {
            tls: Some(TlsRuntime {
                sni_map: registry,
            }),
            routers: HashMap::new(),
            devices: Default::default(),
            services: HashMap::new(),
        };
        SnakewayCertResolver::new(Arc::new(ArcSwap::from_pointee(state)))
    }

    fn make_resolver_without_tls() -> SnakewayCertResolver {
        let state = RuntimeState {
            tls: None,
            routers: HashMap::new(),
            devices: Default::default(),
            services: HashMap::new(),
        };
        SnakewayCertResolver::new(Arc::new(ArcSwap::from_pointee(state)))
    }

    #[test]
    fn lookup_returns_cert_for_known_hostname() {
        // Arrange
        let key = make_test_certified_key();
        let resolver = make_resolver_with_sni(vec![("test.example", key.clone())]);

        // Act
        let result = resolver.lookup("test.example");

        // Assert
        assert!(result.is_some());
    }

    #[test]
    fn lookup_returns_none_for_unknown_hostname() {
        // Arrange
        let key = make_test_certified_key();
        let resolver = make_resolver_with_sni(vec![("test.example", key)]);

        // Act
        let result = resolver.lookup("unknown.example");

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn lookup_returns_none_when_no_tls_runtime() {
        // Arrange
        let resolver = make_resolver_without_tls();

        // Act
        let result = resolver.lookup("test.example");

        // Assert
        assert!(result.is_none());
    }
}
