use crate::runtime::UpstreamId;
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub(crate) enum UpstreamOutcome {
    Transport(TransportFailure),
    HttpStatus(u16),
    Success,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum TransportFailure {
    Connect,
    Timeout,
    Reset,
    Protocol,
    Tls,
}

/// Health state of an upstream endpoint
#[derive(Debug, Clone)]
pub(crate) enum HealthState {
    Healthy,
    Unhealthy {
        consecutive_failures: u64,
        last_failure: Instant,
    },
}

/// Weighted Round Robin state.
#[derive(Debug, Clone)]
pub(crate) struct WrrState {
    // Smooth WRR "current" accumulator per upstream slot.
    pub(crate) current_weights: Vec<i64>,

    // Detect when the healthy set has changed (health flaps, reload, reorder, etc.)
    pub(crate) upstream_ids: Vec<UpstreamId>,
    pub(crate) total_weight: i64,
}
