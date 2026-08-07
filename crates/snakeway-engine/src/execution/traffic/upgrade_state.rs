//! Explicit upgrade negotiation.
//!
//! `UpgradeState` names the lifecycle of an HTTP/1.1 `Upgrade` handshake (a
//! WebSocket) as it unfolds across the proxy hooks. It is seeded at hydration
//! and carried on the request context, and each hook advances it through a
//! transition method rather than mutating flags.
//!
//! The connection pool slot is owned by the states that hold one. The guard
//! releases the slot on Drop, so moving it forward through the transitions
//! preserves the slot until the request context is dropped after `logging`.

use crate::execution::ws_connection_management::WsConnectionGuard;
use http::StatusCode;

/// The state of an upgrade handshake, from request to tunnel or terminal rejection.
#[derive(Debug, Default)]
pub enum UpgradeState {
    /// The request is not an upgrade. The machine never leaves this state.
    #[default]
    NotUpgrade,
    /// A valid `Upgrade` handshake was seen at hydration.
    Requested,
    /// The route allows WebSockets and a pool slot was acquired.
    Admitted { guard: WsConnectionGuard },
    /// The upstream request was shaped for the handshake (HTTP/1.1 forced,
    /// upgrade headers set).
    Negotiated { guard: WsConnectionGuard },
    /// The upstream answered `101` and the tunnel is open.
    Switched { guard: WsConnectionGuard },
    /// The proxy rejected the handshake before contacting an upstream.
    ProxyRejected { status: StatusCode },
    /// The upstream answered the handshake with a status other than `101`,
    /// which is forwarded as a normal response.
    UpstreamRejected {
        guard: WsConnectionGuard,
        status: StatusCode,
    },
    /// The upstream connection failed or aborted before the `101`.
    Failed { guard: WsConnectionGuard },
    /// The tunnel ended, cleanly or through a transport error.
    Closed,
}

impl UpgradeState {
    /// Seeds the machine from the hydrated request.
    pub fn begin(is_upgrade_req: bool) -> Self {
        if is_upgrade_req {
            Self::Requested
        } else {
            Self::NotUpgrade
        }
    }

    /// Requested to Admitted: the route allows WebSockets and a slot was acquired.
    pub fn admit(self, guard: WsConnectionGuard) -> Self {
        match self {
            Self::Requested => Self::Admitted { guard },
            other => Self::invalid(other, "admit"),
        }
    }

    /// Requested to ProxyRejected: the proxy refused the handshake.
    pub fn reject_at_proxy(self, status: StatusCode) -> Self {
        match self {
            Self::Requested => Self::ProxyRejected { status },
            other => Self::invalid(other, "reject_at_proxy"),
        }
    }

    /// Admitted to Negotiated: the upstream request was shaped for the handshake.
    ///
    /// Negotiated to Negotiated is legal because a Pingora retry on a reused
    /// upstream connection re-runs `upstream_request_filter`.
    pub fn negotiate(self) -> Self {
        match self {
            Self::Admitted { guard } | Self::Negotiated { guard } => Self::Negotiated { guard },
            other => Self::invalid(other, "negotiate"),
        }
    }

    /// Negotiated to Switched: the upstream answered `101`.
    pub fn switch(self) -> Self {
        match self {
            Self::Negotiated { guard } => Self::Switched { guard },
            other => Self::invalid(other, "switch"),
        }
    }

    /// Negotiated to UpstreamRejected: the upstream answered something other
    /// than `101`.
    pub fn reject_at_upstream(self, status: StatusCode) -> Self {
        match self {
            Self::Negotiated { guard } => Self::UpstreamRejected { guard, status },
            other => Self::invalid(other, "reject_at_upstream"),
        }
    }

    /// Admitted or Negotiated to Failed: the upstream connection failed or
    /// aborted before the `101`.
    pub fn fail(self) -> Self {
        match self {
            Self::Admitted { guard } | Self::Negotiated { guard } => Self::Failed { guard },
            other => Self::invalid(other, "fail"),
        }
    }

    /// Switched to Closed: either side ended the tunnel.
    pub fn close(self) -> Self {
        match self {
            Self::Switched { .. } => Self::Closed,
            other => Self::invalid(other, "close"),
        }
    }

    /// An invalid transition panics in debug builds and keeps the current
    /// state in release builds, so a missed edge degrades to the guard
    /// releasing at request-context drop rather than a crash.
    fn invalid(state: Self, event: &str) -> Self {
        debug_assert!(false, "invalid upgrade transition: {event} from {state:?}");
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::route::types::RouteId;
    use crate::execution::ws_connection_management::WsConnectionManager;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    fn route_id() -> RouteId {
        RouteId::service("/ws", "test-service")
    }

    /// A manager with a single slot, already acquired by the returned guard.
    fn acquired_slot_at_capacity_one() -> (WsConnectionManager, WsConnectionGuard) {
        let manager = WsConnectionManager::new();
        let guard = manager
            .try_acquire(&route_id(), Some(1))
            .expect("first acquire at capacity 1 must succeed");
        (manager, guard)
    }

    fn slot_is_held(manager: &WsConnectionManager) -> bool {
        match manager.try_acquire(&route_id(), Some(1)) {
            Some(probe) => {
                drop(probe);
                false
            }
            None => true,
        }
    }

    #[test]
    fn should_begin_as_requested_for_an_upgrade() {
        // Arrange
        let is_upgrade_req = true;

        // Act
        let state = UpgradeState::begin(is_upgrade_req);

        // Assert
        assert!(matches!(state, UpgradeState::Requested));
    }

    #[test]
    fn should_begin_as_not_upgrade_otherwise() {
        // Arrange
        let is_upgrade_req = false;

        // Act
        let state = UpgradeState::begin(is_upgrade_req);

        // Assert
        assert!(matches!(state, UpgradeState::NotUpgrade));
    }

    #[test]
    fn should_default_to_not_upgrade_before_hydration() {
        // Act
        let state = UpgradeState::default();

        // Assert
        assert!(matches!(state, UpgradeState::NotUpgrade));
    }

    #[test]
    fn should_move_guard_from_requested_through_switched() {
        // Arrange
        let (manager, guard) = acquired_slot_at_capacity_one();
        let state = UpgradeState::begin(true);

        // Act
        let state = state.admit(guard).negotiate().switch();

        // Assert
        assert!(matches!(state, UpgradeState::Switched { .. }));
        assert!(
            slot_is_held(&manager),
            "the slot must stay held in Switched"
        );
    }

    #[test]
    fn should_keep_status_in_proxy_rejected() {
        // Arrange
        let state = UpgradeState::begin(true);

        // Act
        let state = state.reject_at_proxy(StatusCode::SERVICE_UNAVAILABLE);

        // Assert
        assert!(matches!(
            state,
            UpgradeState::ProxyRejected {
                status: StatusCode::SERVICE_UNAVAILABLE
            }
        ));
    }

    #[test]
    fn should_hold_slot_in_upstream_rejected_until_drop() {
        // Arrange
        let (manager, guard) = acquired_slot_at_capacity_one();
        let state = UpgradeState::begin(true).admit(guard).negotiate();

        // Act
        let state = state.reject_at_upstream(StatusCode::BAD_GATEWAY);

        // Assert
        assert!(matches!(
            state,
            UpgradeState::UpstreamRejected {
                status: StatusCode::BAD_GATEWAY,
                ..
            }
        ));
        assert!(slot_is_held(&manager));
        drop(state);
        assert!(!slot_is_held(&manager), "drop must release the slot");
    }

    #[test]
    fn should_hold_slot_in_failed_until_drop() {
        // Arrange
        let (manager, guard) = acquired_slot_at_capacity_one();
        let state = UpgradeState::begin(true).admit(guard);

        // Act
        let state = state.fail();

        // Assert
        assert!(matches!(state, UpgradeState::Failed { .. }));
        assert!(slot_is_held(&manager));
        drop(state);
        assert!(!slot_is_held(&manager), "drop must release the slot");
    }

    #[test]
    fn should_release_slot_on_close() {
        // Arrange
        let (manager, guard) = acquired_slot_at_capacity_one();
        let state = UpgradeState::begin(true).admit(guard).negotiate().switch();

        // Act
        let state = state.close();

        // Assert
        assert!(matches!(state, UpgradeState::Closed));
        assert!(!slot_is_held(&manager), "close must release the slot");
    }

    #[test]
    fn should_allow_negotiate_again_on_retry() {
        // Arrange
        let (manager, guard) = acquired_slot_at_capacity_one();
        let state = UpgradeState::begin(true).admit(guard).negotiate();

        // Act
        let state = state.negotiate();

        // Assert
        assert!(matches!(state, UpgradeState::Negotiated { .. }));
        assert!(slot_is_held(&manager));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "invalid upgrade transition")]
    fn should_reject_switch_without_negotiation() {
        // Arrange
        let state = UpgradeState::begin(true);

        // Act
        let _ = state.switch();

        // Assert: the debug assertion panics in debug builds.
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Event {
        Admit,
        RejectAtProxy,
        Negotiate,
        Switch,
        RejectAtUpstream,
        Fail,
        Close,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum StateKind {
        NotUpgrade,
        Requested,
        Admitted,
        Negotiated,
        Switched,
        ProxyRejected,
        UpstreamRejected,
        Failed,
        Closed,
    }

    fn make_state(kind: StateKind) -> UpgradeState {
        let build_with_guard = |kind: StateKind| {
            let (_manager, guard) = acquired_slot_at_capacity_one();
            let admitted = UpgradeState::begin(true).admit(guard);
            match kind {
                StateKind::Admitted => admitted,
                StateKind::Negotiated => admitted.negotiate(),
                StateKind::Switched => admitted.negotiate().switch(),
                StateKind::UpstreamRejected => admitted
                    .negotiate()
                    .reject_at_upstream(StatusCode::BAD_GATEWAY),
                StateKind::Failed => admitted.fail(),
                _ => unreachable!(),
            }
        };
        match kind {
            StateKind::NotUpgrade => UpgradeState::NotUpgrade,
            StateKind::Requested => UpgradeState::Requested,
            StateKind::ProxyRejected => UpgradeState::ProxyRejected {
                status: StatusCode::UPGRADE_REQUIRED,
            },
            StateKind::Closed => UpgradeState::Closed,
            with_guard => build_with_guard(with_guard),
        }
    }

    fn kind_of(state: &UpgradeState) -> StateKind {
        match state {
            UpgradeState::NotUpgrade => StateKind::NotUpgrade,
            UpgradeState::Requested => StateKind::Requested,
            UpgradeState::Admitted { .. } => StateKind::Admitted,
            UpgradeState::Negotiated { .. } => StateKind::Negotiated,
            UpgradeState::Switched { .. } => StateKind::Switched,
            UpgradeState::ProxyRejected { .. } => StateKind::ProxyRejected,
            UpgradeState::UpstreamRejected { .. } => StateKind::UpstreamRejected,
            UpgradeState::Failed { .. } => StateKind::Failed,
            UpgradeState::Closed => StateKind::Closed,
        }
    }

    fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_string()))
            .unwrap_or_default()
    }

    fn apply(state: UpgradeState, event: Event) -> UpgradeState {
        let (_manager, spare) = acquired_slot_at_capacity_one();
        match event {
            Event::Admit => state.admit(spare),
            Event::RejectAtProxy => state.reject_at_proxy(StatusCode::UPGRADE_REQUIRED),
            Event::Negotiate => state.negotiate(),
            Event::Switch => state.switch(),
            Event::RejectAtUpstream => state.reject_at_upstream(StatusCode::BAD_GATEWAY),
            Event::Fail => state.fail(),
            Event::Close => state.close(),
        }
    }

    /// The full state-by-event table, so no pair is left undefined.
    #[test]
    fn should_define_every_state_event_pair() {
        // Arrange: the complete set of legal (state, event) pairs.
        let legal: &[(StateKind, Event)] = &[
            (StateKind::Requested, Event::Admit),
            (StateKind::Requested, Event::RejectAtProxy),
            (StateKind::Admitted, Event::Negotiate),
            (StateKind::Admitted, Event::Fail),
            (StateKind::Negotiated, Event::Negotiate),
            (StateKind::Negotiated, Event::Switch),
            (StateKind::Negotiated, Event::RejectAtUpstream),
            (StateKind::Negotiated, Event::Fail),
            (StateKind::Switched, Event::Close),
        ];
        let all_states = [
            StateKind::NotUpgrade,
            StateKind::Requested,
            StateKind::Admitted,
            StateKind::Negotiated,
            StateKind::Switched,
            StateKind::ProxyRejected,
            StateKind::UpstreamRejected,
            StateKind::Failed,
            StateKind::Closed,
        ];
        let all_events = [
            Event::Admit,
            Event::RejectAtProxy,
            Event::Negotiate,
            Event::Switch,
            Event::RejectAtUpstream,
            Event::Fail,
            Event::Close,
        ];

        for state_kind in all_states {
            for event in all_events {
                // Act
                let outcome =
                    catch_unwind(AssertUnwindSafe(|| apply(make_state(state_kind), event)));

                // Assert: a legal pair transitions, an illegal pair trips the
                // debug assertion in debug builds and preserves the state in
                // release builds.
                let expected_legal = legal.contains(&(state_kind, event));
                match outcome {
                    Ok(next) => {
                        if expected_legal {
                            drop(next);
                        } else if cfg!(debug_assertions) {
                            panic!(
                                "pair ({state_kind:?}, {event:?}) must trip the debug assertion"
                            );
                        } else {
                            assert_eq!(
                                kind_of(&next),
                                state_kind,
                                "illegal pair ({state_kind:?}, {event:?}) must preserve the state in release builds"
                            );
                        }
                    }
                    Err(payload) => {
                        let message = panic_message(payload.as_ref());
                        assert!(
                            message.contains("invalid upgrade transition"),
                            "pair ({state_kind:?}, {event:?}) panicked outside the transition: {message}"
                        );
                        assert!(
                            !expected_legal,
                            "legal pair ({state_kind:?}, {event:?}) must not panic"
                        );
                    }
                }
            }
        }
    }
}
