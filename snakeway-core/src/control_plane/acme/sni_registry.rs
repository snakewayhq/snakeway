use crate::control_plane::acme::ParsedCert;
use arc_swap::ArcSwap;
use std::collections::HashMap;
use std::sync::Arc;

pub type SniMap = HashMap<String, Arc<ParsedCert>>;

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

    pub fn publish(&self, map: SniMap) {
        self.inner.store(Arc::new(map));
    }
}
