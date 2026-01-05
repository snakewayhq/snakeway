use crate::connection_management::state::RouteConnectionState;
use std::sync::Arc;

/// RAII guard for a single acquired connection slot.
///
/// Invariants:
/// - A guard is created *only after* RouteConnectionState::try_acquire() succeeds
/// - The slot is released exactly once on Drop
#[derive(Debug)]
pub struct ConnectionGuard {
    state: Arc<RouteConnectionState>,
}

impl ConnectionGuard {
    /// Create a guard for an already-acquired slot.
    /// This is intentionally restricted to prevent bypassing limits.
    pub(crate) fn new_acquired(state: Arc<RouteConnectionState>) -> Self {
        Self { state }
    }
}

impl Drop for ConnectionGuard {
    /// Release the slot when the request ends.
    fn drop(&mut self) {
        self.state.release();
    }
}
