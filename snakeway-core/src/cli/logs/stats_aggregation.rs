use crate::cli::logs::constants::IN_FLIGHT_TTL;
use crate::cli::logs::histogram::{Histogram, percentile_from_histogram};
use crate::cli::logs::types::{IdentitySummary, LogEvent};
use crate::ctx::RequestId;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant, SystemTime};

const LATENCY_BUCKETS_MS: &[u64] = &[1, 5, 10, 25, 50, 100, 250, 500, 1000];

struct WindowEvent {
    inserted_at: Instant,    // for eviction
    latency_ms: Option<u64>, // computed from timestamps when available
    status: Option<i64>,
    identity: IdentitySummary,
}

pub struct StatsAggregator {
    window: Duration,
    events: VecDeque<WindowEvent>,
    in_flight: HashMap<RequestId, InFlight>,
}

struct InFlight {
    start_instant: Instant,           // for TTL eviction
    start_system: Option<SystemTime>, // for latency math
    status: Option<i64>,
    identity: IdentitySummary,
}

impl StatsAggregator {
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            events: VecDeque::new(),
            in_flight: HashMap::new(),
        }
    }

    pub(crate) fn push(&mut self, event: &LogEvent) {
        let LogEvent::Snakeway(e) = event else {
            return;
        };
        let Some(request_id) = &e.request_id else {
            return;
        };

        let request_id = RequestId(request_id.clone());

        match e.name.as_str() {
            "request" => {
                self.in_flight.entry(request_id).or_insert(InFlight {
                    start_instant: Instant::now(),
                    start_system: e.ts,
                    status: None,
                    identity: e.identity.clone().unwrap_or_default(),
                });
            }
            "after_proxy" => {
                if let Some(f) = self.in_flight.get_mut(&request_id) {
                    f.status = e.status;
                }
            }
            "response" => {
                if let Some(f) = self.in_flight.remove(&request_id) {
                    let latency_ms = match (e.ts, f.start_system) {
                        (Some(end), Some(start)) => {
                            end.duration_since(start).ok().map(|d| d.as_millis() as u64)
                        }
                        _ => None,
                    };

                    self.events.push_back(WindowEvent {
                        inserted_at: Instant::now(),
                        latency_ms,
                        status: e.status.or(f.status),
                        identity: f.identity,
                    });
                }
            }
            _ => {}
        }
    }

    fn evict_window(&mut self, now: Instant) {
        while let Some(ev) = self.events.front() {
            if now.duration_since(ev.inserted_at) > self.window {
                self.events.pop_front();
            } else {
                break;
            }
        }
    }

    fn evict_in_flight(&mut self, now: Instant) {
        self.in_flight
            .retain(|_, f| now.duration_since(f.start_instant) <= IN_FLIGHT_TTL);
    }

    pub(crate) fn snapshot(&mut self) -> StatsSnapshot {
        let now = Instant::now();
        self.evict_window(now);
        self.evict_in_flight(now);

        let mut latency = Histogram::new(LATENCY_BUCKETS_MS);
        let mut status_2xx = 0;
        let mut status_4xx = 0;
        let mut status_5xx = 0;

        let mut device_counts: HashMap<String, u64> = HashMap::new();
        let mut bot_count = 0;
        let mut human_count = 0;
        let mut unknown_identity_count = 0;

        for ev in &self.events {
            if let Some(ms) = ev.latency_ms {
                latency.record(ms);
            }

            if let Some(status) = ev.status {
                match status {
                    200..=299 => status_2xx += 1,
                    400..=499 => status_4xx += 1,
                    500..=599 => status_5xx += 1,
                    _ => {}
                }
            }

            match ev.identity.bot {
                Some(true) => bot_count += 1,
                Some(false) => human_count += 1,
                None => unknown_identity_count += 1,
            }

            if let Some(device) = &ev.identity.device {
                *device_counts.entry(device.clone()).or_insert(0) += 1;
            }
        }

        let buckets = latency.numeric_buckets();
        let total_latency: u64 = buckets.iter().map(|(_, c)| *c).sum();

        let p95_ms = percentile_from_histogram(&buckets, total_latency, 0.95);
        let p99_ms = percentile_from_histogram(&buckets, total_latency, 0.99);

        // RPS: use observed span, but avoid lying for sub-second spans by clamping to 0.1s.
        let span = self
            .events
            .back()
            .zip(self.events.front())
            .map(|(b, f)| b.inserted_at.duration_since(f.inserted_at))
            .unwrap_or(self.window);

        let denom = span.as_secs_f64().clamp(0.1, self.window.as_secs_f64());
        let rps = self.events.len() as f64 / denom;

        StatsSnapshot {
            window_seconds: self.window.as_secs().max(1),
            rps,
            window_events: self.events.len() as u64,
            latency: latency.snapshot(),
            status: (status_2xx, status_4xx, status_5xx),
            p95_ms,
            p99_ms,
            device_counts,
            bot_count,
            human_count,
            unknown_identity_count,
        }
    }
}

pub struct StatsSnapshot {
    pub window_seconds: u64,

    pub rps: f64,
    pub window_events: u64,
    pub latency: Vec<(String, u64)>,
    pub status: (u64, u64, u64), // 2xx, 4xx, 5xx

    pub p95_ms: u64,
    pub p99_ms: u64,

    pub device_counts: HashMap<String, u64>,
    pub bot_count: u64,
    pub human_count: u64,
    pub unknown_identity_count: u64,
}
