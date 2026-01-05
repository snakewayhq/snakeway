use crate::connection::ConnectionManager;
use crate::runtime::RuntimeState;
use crate::traffic::TrafficManager;
use arc_swap::{ArcSwap, Guard};
use std::sync::Arc;

pub(crate) struct GatewayCtx {
    state: Arc<ArcSwap<RuntimeState>>,
    pub(crate) traffic_manager: Arc<TrafficManager>,
    pub(crate) connection_manager: Arc<ConnectionManager>,
}

impl GatewayCtx {
    pub(crate) fn new(
        state: Arc<ArcSwap<RuntimeState>>,
        traffic_manager: Arc<TrafficManager>,
        connection_manager: Arc<ConnectionManager>,
    ) -> Self {
        Self {
            state,
            traffic_manager,
            connection_manager,
        }
    }

    pub(crate) fn state(&self) -> Guard<Arc<RuntimeState>> {
        self.state.load()
    }
}
