use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct CircuitBreakerParams {
    pub enabled: bool,
    pub failure_threshold: u32,
    pub open_duration: Duration,
    pub half_open_max_requests: u32,
    pub success_threshold: u32,
    pub count_http_5xx_as_failure: bool,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    // state machine data
    state: CircuitState,

    // Closed
    consecutive_failures: u32,

    // Open
    opened_at: Option<Instant>,

    // HalfOpen
    half_open_in_flight: u32,
    half_open_successes: u32,
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            consecutive_failures: 0,
            opened_at: None,
            half_open_in_flight: 0,
            half_open_successes: 0,
        }
    }

    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// Returns whether we should allow *starting* a request to this upstream right now.
    pub fn allow_request(&mut self, p: &CircuitBreakerParams) -> bool {
        if !p.enabled {
            return true;
        }

        match self.state {
            CircuitState::Closed => true,

            CircuitState::Open => {
                let opened_at = match self.opened_at {
                    Some(t) => t,
                    None => {
                        // Shouldn't happen, but fail safe: treat as open.
                        return false;
                    }
                };

                if opened_at.elapsed() >= p.open_duration {
                    // Promote to half-open and allow probes.
                    self.state = CircuitState::HalfOpen;
                    self.opened_at = None;
                    self.half_open_in_flight = 0;
                    self.half_open_successes = 0;

                    // fall-through to half-open logic
                    self.allow_request(p)
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
    pub fn on_request_end(&mut self, p: &CircuitBreakerParams, started: bool, success: bool) {
        if !p.enabled {
            return;
        }

        match self.state {
            CircuitState::Closed => {
                if success {
                    self.consecutive_failures = 0;
                } else {
                    self.consecutive_failures = self.consecutive_failures.saturating_add(1);
                    if self.consecutive_failures >= p.failure_threshold {
                        self.trip_open();
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
                        self.reset_closed();
                    }
                } else {
                    // Any failure while half-open should immediately re-open.
                    self.trip_open();
                }
            }
        }
    }

    fn trip_open(&mut self) {
        self.state = CircuitState::Open;
        self.opened_at = Some(Instant::now());
        self.consecutive_failures = 0;
        self.half_open_in_flight = 0;
        self.half_open_successes = 0;
    }

    fn reset_closed(&mut self) {
        self.state = CircuitState::Closed;
        self.opened_at = None;
        self.consecutive_failures = 0;
        self.half_open_in_flight = 0;
        self.half_open_successes = 0;
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}
