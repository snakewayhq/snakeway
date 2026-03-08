use crate::control_plane::acme::ParsedCert;
use arc_swap::ArcSwap;
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) type SniMap = HashMap<String, Arc<ParsedCert>>;

pub(crate) struct SniRegistry {
    inner: ArcSwap<SniMap>,
}

impl SniRegistry {
    pub(crate) fn new(initial: SniMap) -> Self {
        Self {
            inner: ArcSwap::from_pointee(initial),
        }
    }

    pub(crate) fn load(&self) -> Arc<SniMap> {
        self.inner.load_full()
    }

    pub(crate) fn publish(&self, map: SniMap) {
        self.inner.store(Arc::new(map));
    }
}
