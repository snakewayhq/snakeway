use arc_swap::{ArcSwap, Guard};
use snakeway_engine::WsConnectionManager;
use snakeway_engine::runtime::RuntimeState;
use snakeway_engine::traffic::TrafficManager;
use snakeway_observability::Metrics;
use std::sync::Arc;

pub(crate) struct ProxyCtx {
    state: Arc<ArcSwap<RuntimeState>>,
    pub(crate) traffic_manager: Arc<TrafficManager>,
    pub(crate) connection_manager: Arc<WsConnectionManager>,
    pub(crate) metrics: Option<Arc<Metrics>>,
}

impl ProxyCtx {
    pub(crate) fn new(
        state: Arc<ArcSwap<RuntimeState>>,
        traffic_manager: Arc<TrafficManager>,
        connection_manager: Arc<WsConnectionManager>,
        metrics: Option<Arc<Metrics>>,
    ) -> Self {
        Self {
            state,
            traffic_manager,
            connection_manager,
            metrics,
        }
    }

    pub(crate) fn state(&self) -> Guard<Arc<RuntimeState>> {
        self.state.load()
    }
}
