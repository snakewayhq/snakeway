use crate::traffic::snapshot::TrafficSnapshot;
use arc_swap::ArcSwap;
use std::sync::Arc;

pub struct TrafficManager {
    snapshot: ArcSwap<TrafficSnapshot>,
}

impl TrafficManager {
    pub fn new(initial: TrafficSnapshot) -> Self {
        Self {
            snapshot: ArcSwap::from_pointee(initial),
        }
    }

    pub fn snapshot(&self) -> Arc<TrafficSnapshot> {
        self.snapshot.load_full()
    }

    pub fn update(&self, new_snapshot: TrafficSnapshot) {
        self.snapshot.store(Arc::new(new_snapshot));
    }
}
