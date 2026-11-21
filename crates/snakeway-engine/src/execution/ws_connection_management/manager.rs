use crate::execution::route::types::RouteId;
use crate::execution::ws_connection_management::guard::WsConnectionGuard;
use crate::execution::ws_connection_management::state::WsRouteConnectionState;
use dashmap::DashMap;
use serde::Serialize;
use std::sync::Arc;

/// Global registry of per-route connection state.
///
/// Lives beside the runtime state and survives reloads.
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

    /// Get (or lazily create) the connection state for a route, and store the
    /// limit in force now, so a reload that changed it applies to
    /// acquisitions from this call onward.
    pub(crate) fn route_state(
        &self,
        route_id: &RouteId,
        max: Option<usize>,
    ) -> Arc<WsRouteConnectionState> {
        match self.routes.entry(route_id.clone()) {
            dashmap::mapref::entry::Entry::Vacant(entry) => entry
                .insert(Arc::new(WsRouteConnectionState::new(max)))
                .clone(),
            dashmap::mapref::entry::Entry::Occupied(entry) => {
                entry.get().set_max(max);
                entry.get().clone()
            }
        }
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

    /// Drop the state of routes the predicate does not keep, unless they still
    /// hold active connections, which a later call can drop once drained.
    pub fn retain(&self, keep: impl Fn(&RouteId) -> bool) {
        self.routes
            .retain(|route_id, state| keep(route_id) || state.active() > 0);
    }
}

#[derive(Debug, Serialize)]
pub struct RouteConnectionSnapshot {
    pub(crate) route_id: RouteId,
    pub(crate) active: usize,
    pub(crate) max: Option<usize>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn service_route(path: &str) -> RouteId {
        RouteId::service(path, "api")
    }

    #[test]
    fn route_state_updates_the_limit_of_an_existing_route() {
        // Arrange
        let manager = WsConnectionManager::new();
        let route = service_route("/ws");
        manager.route_state(&route, Some(1));

        // Act
        manager.route_state(&route, Some(5));

        // Assert
        let snapshot = manager.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].max, Some(5));
    }

    #[test]
    fn retain_drops_drained_routes_the_live_set_does_not_keep() {
        // Arrange
        let manager = WsConnectionManager::new();
        let kept = service_route("/kept");
        let removed = service_route("/removed");
        manager.route_state(&kept, None);
        manager.route_state(&removed, None);
        let live: std::collections::HashSet<RouteId> = [kept.clone()].into();

        // Act
        manager.retain(|route_id| live.contains(route_id));

        // Assert
        let snapshot = manager.snapshot();
        let ids: Vec<&RouteId> = snapshot.iter().map(|s| &s.route_id).collect();
        assert_eq!(ids, vec![&kept]);
    }

    #[test]
    fn retain_keeps_a_removed_route_until_its_connections_drain() {
        // Arrange
        let manager = WsConnectionManager::new();
        let live = service_route("/live");
        let removed = service_route("/removed");
        manager.route_state(&live, None);
        let guard = manager.try_acquire(&removed, None).unwrap();
        let live_set: std::collections::HashSet<RouteId> = [live.clone()].into();

        // Act
        manager.retain(|route_id| live_set.contains(route_id));

        // Assert: the removed route still has a connection, so it stays.
        assert_eq!(manager.snapshot().len(), 2);
        drop(guard);

        // Act again: drained, so the next retain drops it.
        manager.retain(|route_id| live_set.contains(route_id));

        // Assert
        let snapshot = manager.snapshot();
        let ids: Vec<&RouteId> = snapshot.iter().map(|s| &s.route_id).collect();
        assert_eq!(ids, vec![&live]);
    }
}
