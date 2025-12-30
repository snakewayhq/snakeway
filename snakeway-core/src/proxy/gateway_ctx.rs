use crate::server::RuntimeState;
use crate::traffic::{TrafficDirector, TrafficManager};
use arc_swap::{ArcSwap, Guard};
use std::sync::Arc;

pub(crate) struct GatewayCtx {
    state: Arc<ArcSwap<RuntimeState>>,
    pub(crate) traffic_manager: Arc<TrafficManager>,
    pub(crate) traffic_director: TrafficDirector,
}

impl GatewayCtx {
    pub(crate) fn new(
        state: Arc<ArcSwap<RuntimeState>>,
        traffic_manager: Arc<TrafficManager>,
    ) -> Self {
        Self {
            state,
            traffic_manager,
            traffic_director: TrafficDirector,
        }
    }

    pub(crate) fn state(&self) -> Guard<Arc<RuntimeState>> {
        self.state.load()
    }
}
