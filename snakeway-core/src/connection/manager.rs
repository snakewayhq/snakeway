use crate::connection::guard::ConnectionGuard;
use crate::connection::state::RouteConnectionState;
use crate::route::types::RouteId;
use dashmap::DashMap;
use std::sync::Arc;

/// Global registry of per-route connection state.
///
/// Lives in runtime state and survives reloads.
#[derive(Debug, Default)]
pub struct ConnectionManager {
    routes: DashMap<RouteId, Arc<RouteConnectionState>>,
}

impl ConnectionManager {
    /// Create a new, empty manager.
    pub fn new() -> Self {
        Self {
            routes: DashMap::new(),
        }
    }

    /// Get (or lazily create) the connection state for a route.
    ///
    /// `max` is applied only on first creation and is immutable thereafter.
    pub fn route_state(&self, route_id: &RouteId, max: Option<usize>) -> Arc<RouteConnectionState> {
        self.routes
            .entry(route_id.clone())
            .or_insert_with(|| Arc::new(RouteConnectionState::new(max)))
            .clone()
    }

    /// Attempt to acquire a connection slot for the given route.
    ///
    /// On success, returns a ConnectionGuard that will release the slot on Drop.
    pub fn try_acquire(&self, route_id: &RouteId, max: Option<usize>) -> Option<ConnectionGuard> {
        let state = self.route_state(route_id, max);

        if !state.try_acquire() {
            return None;
        }

        Some(ConnectionGuard::new_acquired(state))
    }

    /// Get the current active connection count for a route.
    /// Intended for admin / observability.
    pub fn active(&self, route_id: &RouteId) -> usize {
        self.routes
            .get(route_id)
            .map(|state| state.active())
            .unwrap_or(0)
    }
}
