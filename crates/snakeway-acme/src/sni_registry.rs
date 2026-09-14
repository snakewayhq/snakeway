use arc_swap::ArcSwap;
use pingora_rustls::sign;
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) type SniMap = HashMap<String, Arc<sign::CertifiedKey>>;

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
