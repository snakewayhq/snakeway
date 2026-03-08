use crate::route::types::RouteId;
use crate::ws_connection_management::guard::WsConnectionGuard;
use crate::ws_connection_management::state::WsRouteConnectionState;
use dashmap::DashMap;
use std::sync::Arc;

/// Global registry of per-route connection state.
///
/// Lives in runtime state and survives reloads.
#[derive(Debug, Default)]
pub struct WsConnectionManager {
    routes: DashMap<RouteId, Arc<WsRouteConnectionState>>,
}

impl WsConnectionManager {
    /// Create a new, empty manager.
    pub fn new() -> Self {
        Self {
            routes: DashMap::new(),
        }
    }

    /// Get (or lazily create) the connection state for a route.
    ///
    /// `max` is applied only on the first creation and is immutable thereafter.
    pub fn route_state(
        &self,
        route_id: &RouteId,
        max: Option<usize>,
    ) -> Arc<WsRouteConnectionState> {
        self.routes
            .entry(route_id.clone())
            .or_insert_with(|| Arc::new(WsRouteConnectionState::new(max)))
            .clone()
    }

    /// Attempt to acquire a connection slot for the given route.
    ///
    /// On success, returns a ConnectionGuard that will release the slot on Drop.
    pub fn try_acquire(&self, route_id: &RouteId, max: Option<usize>) -> Option<WsConnectionGuard> {
        let state = self.route_state(route_id, max);

        if !state.try_acquire() {
            return None;
        }

        Some(WsConnectionGuard::new_acquired(state))
    }

    /// Get the current active connection count for a route.
    /// Intended for admin/observability.
    pub fn active(&self, route_id: &RouteId) -> usize {
        self.routes
            .get(route_id)
            .map(|state| state.active())
            .unwrap_or(0)
    }
}

pub struct RouteConnectionSnapshot {
    pub route_id: RouteId,
    pub active: usize,
    pub max: Option<usize>,
}

impl WsConnectionManager {
    pub fn snapshot(&self) -> Vec<RouteConnectionSnapshot> {
        self.routes
            .iter()
            .map(|entry| {
                let state = entry.value();
                RouteConnectionSnapshot {
                    route_id: entry.key().clone(),
                    active: state.active(),
                    max: state.max(),
                }
            })
            .collect()
    }
}
