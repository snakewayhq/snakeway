use crate::runtime::UpstreamId;
use crate::traffic_management::ServiceId;
use std::time::{Duration, Instant, SystemTime};
use tracing::info;

#[derive(Debug, Clone)]
pub struct CircuitBreakerParams {
    pub enable_auto_recovery: bool,
    pub failure_threshold: u32,
    pub open_duration: Duration,
    pub half_open_max_requests: u32,
    pub success_threshold: u32,
    pub count_http_5xx_as_failure: bool,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    // state machine data
    pub(crate) state: CircuitState,

    // Closed
    pub(crate) consecutive_failures: u32,

    // Open
    pub(crate) opened_at_instant: Option<Instant>,
    pub(crate) opened_at_system: Option<SystemTime>,

    // HalfOpen
    pub(crate) half_open_in_flight: u32,
    pub(crate) half_open_successes: u32,
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            consecutive_failures: 0,
            opened_at_instant: None,
            opened_at_system: None,
            half_open_in_flight: 0,
            half_open_successes: 0,
        }
    }

    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// Returns whether we should allow *starting* a request to this upstream right now.
    pub fn allow_request(
        &mut self,
        ids: (&ServiceId, &UpstreamId),
        p: &CircuitBreakerParams,
    ) -> bool {
        match self.state {
            CircuitState::Closed => true,

            CircuitState::Open => {
                let opened_at = match self.opened_at_instant {
                    Some(t) => t,
                    None => {
                        // Shouldn't happen, but failsafe: treat as open.
                        return false;
                    }
                };

                if opened_at.elapsed() >= p.open_duration {
                    // Circuit disabled = no auto recovery
                    if !p.enable_auto_recovery {
                        // remain open until external reset (health)
                        return false;
                    }

                    // Promote to half-open and allow probes.
                    let old_state = self.state;
                    self.state = CircuitState::HalfOpen;
                    self.opened_at_instant = None;
                    self.opened_at_system = None;
                    self.half_open_in_flight = 0;
                    self.half_open_successes = 0;

                    info!(
                        event = "circuit_transition",
                        service = %ids.0,
                        upstream = ?ids.1,
                        from = ?old_state,
                        to = ?self.state,
                        reason = "cooldown_expired"
                    );

                    // fall-through to half-open logic
                    self.allow_request(ids, p)
                } else {
                    false
                }
            }

            CircuitState::HalfOpen => {
                if self.half_open_in_flight < p.half_open_max_requests {
                    self.half_open_in_flight += 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Called when the request finishes. `started` tells us whether this request was actually
    /// admitted by `allow_request()` (so we can unwind counters safely).
    pub fn on_request_end(
        &mut self,
        ids: (&ServiceId, &UpstreamId),
        p: &CircuitBreakerParams,
        started: bool,
        success: bool,
    ) {
        if !p.enable_auto_recovery {
            return;
        }

        match self.state {
            CircuitState::Closed => {
                if success {
                    self.consecutive_failures = 0;
                } else {
                    self.consecutive_failures = self.consecutive_failures.saturating_add(1);
                    if self.consecutive_failures >= p.failure_threshold {
                        self.trip_open(ids, p, "failure_threshold_exceeded");
                    }
                }
            }

            CircuitState::Open => {
                // no-op; we don't admit requests during open
                // except the implicit "time-based promotion" handled in allow_request()
            }

            CircuitState::HalfOpen => {
                if started && self.half_open_in_flight > 0 {
                    self.half_open_in_flight -= 1;
                }

                if success {
                    self.half_open_successes = self.half_open_successes.saturating_add(1);
                    if self.half_open_successes >= p.success_threshold {
                        self.reset_closed(ids, p);
                    }
                } else {
                    // Any failure while half-open should immediately re-open.
                    self.trip_open(ids, p, "half_open_failure");
                }
            }
        }
    }

    pub(crate) fn trip_open(
        &mut self,
        ids: (&ServiceId, &UpstreamId),
        _p: &CircuitBreakerParams,
        reason: &'static str,
    ) {
        let old_state = self.state;
        self.state = CircuitState::Open;
        self.opened_at_instant = Some(Instant::now());
        self.opened_at_system = Some(SystemTime::now());
        let failures = self.consecutive_failures;
        self.consecutive_failures = 0;
        self.half_open_in_flight = 0;
        self.half_open_successes = 0;

        info!(
            event = "circuit_transition",
            service = %ids.0,
            upstream = ?ids.1,
            from = ?old_state,
            to = ?self.state,
            reason = reason,
            failures = failures
        );
    }

    fn reset_closed(&mut self, ids: (&ServiceId, &UpstreamId), _p: &CircuitBreakerParams) {
        let old_state = self.state;
        self.state = CircuitState::Closed;
        self.opened_at_instant = None;
        self.opened_at_system = None;
        self.consecutive_failures = 0;
        self.half_open_in_flight = 0;
        self.half_open_successes = 0;

        info!(
            event = "circuit_transition",
            service = %ids.0,
            upstream = ?ids.1,
            from = ?old_state,
            to = ?self.state,
            reason = "success_threshold_reached"
        );
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}
