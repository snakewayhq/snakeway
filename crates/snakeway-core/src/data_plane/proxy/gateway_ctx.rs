#[cfg(feature = "otel")]
use crate::control_plane::observability::Metrics;
use crate::data_plane::ws_connection_management::WsConnectionManager;
use crate::execution::traffic::TrafficManager;
use crate::runtime::RuntimeState;
use arc_swap::{ArcSwap, Guard};
use std::sync::Arc;

pub(crate) struct GatewayCtx {
    state: Arc<ArcSwap<RuntimeState>>,
    pub(crate) traffic_manager: Arc<TrafficManager>,
    pub(crate) connection_manager: Arc<WsConnectionManager>,
    #[cfg(feature = "otel")]
    pub(crate) metrics: Option<Arc<Metrics>>,
}

impl GatewayCtx {
    pub(crate) fn new(
        state: Arc<ArcSwap<RuntimeState>>,
        traffic_manager: Arc<TrafficManager>,
        connection_manager: Arc<WsConnectionManager>,
        #[cfg(feature = "otel")] metrics: Option<Arc<Metrics>>,
    ) -> Self {
        Self {
            state,
            traffic_manager,
            connection_manager,
            #[cfg(feature = "otel")]
            metrics,
        }
    }

    pub(crate) fn state(&self) -> Guard<Arc<RuntimeState>> {
        self.state.load()
    }
}
