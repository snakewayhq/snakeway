use crate::cli::logs::constants::IN_FLIGHT_TTL;
use crate::cli::logs::histogram::{Histogram, percentile_from_histogram};
use crate::cli::logs::types::{IdentitySummary, LogEvent};
use snakeway_engine::ctx::RequestId;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant, SystemTime};

const LATENCY_BUCKETS_MS: &[u64] = &[1, 5, 10, 25, 50, 100, 250, 500, 1000];

struct WindowEvent {
    inserted_at: Instant,    // for eviction
    latency_ms: Option<u64>, // computed from timestamps when available
    status: Option<i64>,
    identity: IdentitySummary,
}

pub(crate) struct StatsAggregator {
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
    pub(crate) fn new(window: Duration) -> Self {
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

        let request_id = RequestId::from(request_id.clone());

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
        let mut connection_type_counts: HashMap<String, u64> = HashMap::new();
        let mut asn_counts: HashMap<usize, u64> = HashMap::new();
        let mut aso_counts: HashMap<String, u64> = HashMap::new();
        let mut country_counts: HashMap<String, u64> = HashMap::new();
        let mut bot_count = 0;
        let mut human_count = 0;
        let mut unknown_identity_count = 0;

        // Iterate over events and gather the stats for the windowed snapshot.
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

            if let Some(asn) = &ev.identity.asn {
                *asn_counts.entry(*asn).or_insert(0) += 1;
            }

            if let Some(aso) = &ev.identity.aso {
                *aso_counts.entry(aso.clone()).or_insert(0) += 1;
            }

            if let Some(country) = &ev.identity.country {
                *country_counts.entry(country.clone()).or_insert(0) += 1;
            }

            if let Some(connection_type) = &ev.identity.connection_type {
                *connection_type_counts
                    .entry(connection_type.clone())
                    .or_insert(0) += 1;
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

        let denom = span
            .as_secs_f64()
            .clamp(0.1, self.window.as_secs_f64().max(0.1));
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
            connection_type_counts,
            asn_counts,
            aso_counts,
            country_counts,
            bot_count,
            human_count,
            unknown_identity_count,
        }
    }
}

pub(crate) struct StatsSnapshot {
    pub(crate) window_seconds: u64,

    pub(crate) rps: f64,
    pub(crate) window_events: u64,
    pub(crate) latency: Vec<(String, u64)>,
    pub(crate) status: (u64, u64, u64), // 2xx, 4xx, 5xx

    pub(crate) p95_ms: u64,
    pub(crate) p99_ms: u64,

    pub(crate) device_counts: HashMap<String, u64>,
    pub(crate) connection_type_counts: HashMap<String, u64>,
    pub(crate) asn_counts: HashMap<usize, u64>,
    pub(crate) aso_counts: HashMap<String, u64>,
    pub(crate) country_counts: HashMap<String, u64>,
    pub(crate) bot_count: u64,
    pub(crate) human_count: u64,
    pub(crate) unknown_identity_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::logs::types::SnakewayEvent;
    use pretty_assertions::assert_eq;

    fn event(name: &str, request_id: &str, status: Option<i64>, ts_ms: u64) -> LogEvent {
        event_with_identity(name, request_id, status, ts_ms, None)
    }

    fn event_with_identity(
        name: &str,
        request_id: &str,
        status: Option<i64>,
        ts_ms: u64,
        identity: Option<IdentitySummary>,
    ) -> LogEvent {
        LogEvent::Snakeway(SnakewayEvent {
            request_id: Some(request_id.to_string()),
            level: "INFO".to_string(),
            name: name.to_string(),
            method: None,
            uri: None,
            status,
            ts: Some(SystemTime::UNIX_EPOCH + Duration::from_millis(ts_ms)),
            identity,
        })
    }

    fn completed_request(
        aggregator: &mut StatsAggregator,
        request_id: &str,
        status: Option<i64>,
        latency_ms: u64,
    ) {
        aggregator.push(&event("request", request_id, None, 1_000));
        aggregator.push(&event("response", request_id, status, 1_000 + latency_ms));
    }

    #[test]
    fn should_measure_latency_from_request_to_response() {
        // Arrange
        let mut aggregator = StatsAggregator::new(Duration::from_secs(60));
        completed_request(&mut aggregator, "r1", Some(200), 50);

        // Act
        let snapshot = aggregator.snapshot();

        // Assert
        assert_eq!(snapshot.window_events, 1);
        assert_eq!(snapshot.status, (1, 0, 0));
        assert_eq!(
            snapshot.p95_ms, 50,
            "a single 50ms sample lands in the 50ms bucket"
        );
        assert!(snapshot.latency.contains(&("26–50ms".to_string(), 1)));
    }

    #[test]
    fn should_classify_status_ranges() {
        // Arrange
        let mut aggregator = StatsAggregator::new(Duration::from_secs(60));
        completed_request(&mut aggregator, "r1", Some(204), 1);
        completed_request(&mut aggregator, "r2", Some(404), 1);
        completed_request(&mut aggregator, "r3", Some(503), 1);
        completed_request(&mut aggregator, "r4", Some(302), 1);

        // Act
        let snapshot = aggregator.snapshot();

        // Assert
        assert_eq!(snapshot.status, (1, 1, 1), "3xx must not be counted");
        assert_eq!(snapshot.window_events, 4);
    }

    #[test]
    fn should_take_status_from_after_proxy_when_response_lacks_it() {
        // Arrange
        let mut aggregator = StatsAggregator::new(Duration::from_secs(60));
        aggregator.push(&event("request", "r1", None, 1_000));
        aggregator.push(&event("after_proxy", "r1", Some(503), 1_010));
        aggregator.push(&event("response", "r1", None, 1_020));

        // Act
        let snapshot = aggregator.snapshot();

        // Assert
        assert_eq!(snapshot.status, (0, 0, 1));
    }

    #[test]
    fn should_ignore_a_response_without_a_request() {
        // Arrange
        let mut aggregator = StatsAggregator::new(Duration::from_secs(60));
        aggregator.push(&event("response", "orphan", Some(200), 1_000));

        // Act
        let snapshot = aggregator.snapshot();

        // Assert
        assert_eq!(snapshot.window_events, 0);
    }

    #[test]
    fn should_count_identity_summaries() {
        // Arrange
        let mut aggregator = StatsAggregator::new(Duration::from_secs(60));
        let bot = IdentitySummary {
            bot: Some(true),
            device: Some("crawler".to_string()),
            asn: Some(64512),
            aso: Some("TestNet".to_string()),
            connection_type: Some("cellular".to_string()),
            country: Some("NZ".to_string()),
        };
        let human = IdentitySummary {
            bot: Some(false),
            device: Some("phone".to_string()),
            ..Default::default()
        };
        aggregator.push(&event_with_identity(
            "request",
            "r1",
            None,
            1_000,
            Some(bot),
        ));
        aggregator.push(&event("response", "r1", Some(200), 1_001));
        aggregator.push(&event_with_identity(
            "request",
            "r2",
            None,
            1_000,
            Some(human),
        ));
        aggregator.push(&event("response", "r2", Some(200), 1_001));
        completed_request(&mut aggregator, "r3", Some(200), 1);

        // Act
        let snapshot = aggregator.snapshot();

        // Assert
        assert_eq!(snapshot.bot_count, 1);
        assert_eq!(snapshot.human_count, 1);
        assert_eq!(snapshot.unknown_identity_count, 1);
        assert_eq!(snapshot.device_counts.get("crawler"), Some(&1));
        assert_eq!(snapshot.device_counts.get("phone"), Some(&1));
        assert_eq!(snapshot.asn_counts.get(&64512), Some(&1));
        assert_eq!(snapshot.aso_counts.get("TestNet"), Some(&1));
        assert_eq!(snapshot.connection_type_counts.get("cellular"), Some(&1));
        assert_eq!(snapshot.country_counts.get("NZ"), Some(&1));
    }

    #[test]
    fn should_compute_rps_from_the_clamped_span() {
        // Arrange
        let mut aggregator = StatsAggregator::new(Duration::from_secs(60));
        completed_request(&mut aggregator, "r1", Some(200), 1);

        // Act
        let snapshot = aggregator.snapshot();

        // Assert: one event over the 0.1s clamped floor.
        assert!(
            (snapshot.rps - 10.0).abs() < 1e-9,
            "rps was {}",
            snapshot.rps
        );
    }

    #[test]
    fn should_evict_events_older_than_the_window() {
        // Arrange
        let mut aggregator = StatsAggregator::new(Duration::from_millis(50));
        completed_request(&mut aggregator, "r1", Some(200), 1);
        std::thread::sleep(Duration::from_millis(120));

        // Act
        let snapshot = aggregator.snapshot();

        // Assert
        assert_eq!(snapshot.window_events, 0);
    }
}
