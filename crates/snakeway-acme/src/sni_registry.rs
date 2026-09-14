use arc_swap::ArcSwap;
use pingora_rustls::sign;
use std::collections::HashMap;
use std::sync::Arc;

/// Maps each domain name to the TLS certificate chain and private key that
/// the proxy presents during the handshake for that domain.
/// The leaf certificate is first in the chain.
pub(crate) type SniMap = HashMap<String, Arc<sign::CertifiedKey>>;

/// Thread-safe, lock-free registry of domain-to-certificate mappings.
/// Updated atomically when ACME issues or renews a certificate.
pub struct SniRegistry {
    inner: ArcSwap<SniMap>,
}

impl SniRegistry {
    pub fn new(initial: SniMap) -> Self {
        Self {
            inner: ArcSwap::from_pointee(initial),
        }
    }

    pub fn load(&self) -> Arc<SniMap> {
        self.inner.load_full()
    }

    pub(crate) fn publish(&self, map: SniMap) {
        self.inner.store(Arc::new(map));
    }
}
