//! Log Processing Pipeline
//!
//! This module handles reading and displaying log messages from Snakeway in different ways.
//!
//! Think of it like a filter for a stream of water - log messages flow in from the input,
//! and this code decides how to show them to you based on what you asked for.
//!
//! There are three ways to view logs:
//! - **Raw mode**: Shows the logs exactly as they come in, no changes
//! - **Pretty mode**: Makes the logs easier to read by formatting them nicely
//! - **Stats mode**: Instead of showing every single log, it counts things up and shows
//!   you a summary every second - like how many requests per second, how fast they were,
//!   and whether anything went wrong
//!
//! The code reads log messages one line at a time, figures out what kind of message it is
//! (either a web request or a general system message), and then either displays it nicely
//! or adds it to the running statistics counter.
//!
//! For stats mode, it keeps track of recent events in a sliding time window (like looking
//! at the last 10 seconds) and calculates things like request speed and response times.
//!
//!
//! The overall data processing architecture is:
//!
//! stdin
//! parse_event
//! LogEvent
//! StatsAggregator
//! StatsSnapshot
//! render_stats
//!

use crate::ctx::RequestId;
use crate::logging::LogMode;
use anyhow::Result;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::io::{self, BufRead};
use std::time::SystemTime;
use std::time::{Duration, Instant};

pub fn run_logs(mode: LogMode) -> Result<()> {
    let stdin = io::stdin();
    let reader = stdin.lock();

    let mut stats =
        matches!(mode, LogMode::Stats).then(|| StatsAggregator::new(Duration::from_secs(10)));

    let mut last_render = Instant::now();

    for line in reader.lines() {
        let line = line?;

        if matches!(mode, LogMode::Raw) {
            println!("{line}");
            continue;
        }

        let Ok(json) = serde_json::from_str::<Value>(&line) else {
            println!("{line}");
            continue;
        };

        if let Some(event) = parse_event(&json) {
            // Existing behavior
            if matches!(mode, LogMode::Pretty) {
                handle_event(event.clone(), mode);
            }

            // Stats-only path
            if let Some(agg) = stats.as_mut() {
                agg.push(&event);

                if last_render.elapsed() >= Duration::from_secs(1) {
                    let snap = agg.snapshot();
                    render_stats(&snap);
                    last_render = Instant::now();
                }
            }
        }
    }

    Ok(())
}

//-----------------------------------------------------------------------------
// Event model
//-----------------------------------------------------------------------------

#[derive(Clone)]
enum LogEvent {
    Snakeway(SnakewayEvent),
    Generic(GenericEvent),
}

#[derive(Clone)]
struct SnakewayEvent {
    request_id: Option<String>,
    level: String,
    name: String,
    method: Option<String>,
    uri: Option<String>,
    status: Option<i64>,
    ts: SystemTime,
}

#[derive(Clone)]
struct GenericEvent {
    level: String,
    message: String,
    target: Option<String>,
}

//-----------------------------------------------------------------------------
// Parsing
//-----------------------------------------------------------------------------

fn is_snakeway_event(event: &Value) -> bool {
    event.get("method").is_some() || event.get("uri").is_some() || event.get("status").is_some()
}

fn parse_event(event: &Value) -> Option<LogEvent> {
    let level = event
        .get("level")
        .and_then(Value::as_str)
        .unwrap_or("INFO")
        .to_string();

    if is_snakeway_event(event) {
        let request_id = event
            .get("request_id")
            .and_then(Value::as_str)
            .map(str::to_string);
        let ts = event
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(SystemTime::from)
            .unwrap_or(SystemTime::now());
        Some(LogEvent::Snakeway(SnakewayEvent {
            request_id,
            ts,
            level,
            name: event
                .get("event")
                .and_then(Value::as_str)
                .unwrap_or("request")
                .to_string(),
            method: event
                .get("method")
                .and_then(Value::as_str)
                .map(str::to_string),
            uri: event.get("uri").and_then(Value::as_str).map(str::to_string),
            status: event
                .get("status")
                .and_then(Value::as_str)
                .and_then(|s| s.parse::<i64>().ok()),
        }))
    } else {
        Some(LogEvent::Generic(GenericEvent {
            level,
            message: event
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("<no message>")
                .to_string(),
            target: event
                .get("target")
                .and_then(Value::as_str)
                .map(str::to_string),
        }))
    }
}

//-----------------------------------------------------------------------------
// Handling / Rendering
//-----------------------------------------------------------------------------

fn handle_event(event: LogEvent, mode: LogMode) {
    if matches!(mode, LogMode::Pretty) {
        render_pretty(event);
    }
}

fn render_pretty(event: LogEvent) {
    match event {
        LogEvent::Snakeway(e) => {
            print!("[{}] {}", e.level, e.name);
            if let (Some(m), Some(u)) = (&e.method, &e.uri) {
                print!(" → {m} {u}");
            }
            if let Some(s) = e.status {
                print!(" ({s})");
            }
            println!();
        }

        LogEvent::Generic(e) => {
            if let Some(target) = e.target {
                println!("[{}] {} ({})", e.level, e.message, target);
            } else {
                println!("[{}] {}", e.level, e.message);
            }
        }
    }
}

//-----------------------------------------------------------------------------
// Stats Aggregation
//-----------------------------------------------------------------------------

fn render_stats(snapshot: &StatsSnapshot) {
    let (ok, client, server) = snapshot.status;

    println!(
        "RPS: {:.1} | window: {} | 5xx: {}",
        snapshot.rps, snapshot.window_events, server
    );

    let total_latency: u64 = snapshot.latency.iter().map(|(_, c)| *c).sum();
    if total_latency > 0 {
        println!("Latency (window):");
        for (label, count) in &snapshot.latency {
            let pct = (*count as f64 / total_latency as f64) * 100.0;
            let bars = if *count == 0 {
                0
            } else {
                ((pct / 5.0).floor() as usize).max(1)
            };
            println!("  {:<8} {:<20} {:>5.1}%", label, "█".repeat(bars), pct);
        }
    }

    let (ok, client, server) = snapshot.status;
    println!("Status: 2xx={} 4xx={} 5xx={}", ok, client, server);

    println!();
}

struct WindowEvent {
    ts: Instant,
    latency_ms: Option<u64>,
    status: Option<i64>,
}

struct StatsAggregator {
    window: Duration,
    events: VecDeque<WindowEvent>,
    in_flight: HashMap<RequestId, InFlight>,
}

struct InFlight {
    start: Instant,
    status: Option<i64>,
}

impl StatsAggregator {
    fn new(window: Duration) -> Self {
        Self {
            window,
            events: VecDeque::new(),
            in_flight: HashMap::new(),
        }
    }
    fn push(&mut self, event: &LogEvent) {
        let LogEvent::Snakeway(e) = event else {
            return;
        };

        let now = systemtime_to_instant(e.ts);

        let Some(request_id) = &e.request_id else {
            return;
        };

        let request_id = RequestId(request_id.clone());

        match e.name.as_str() {
            //------------------------------------------------------------
            // Request start
            //------------------------------------------------------------
            "request" | "before_proxy" => {
                self.in_flight
                    .entry(request_id.clone())
                    .or_insert(InFlight {
                        start: now,
                        status: None,
                    });
            }

            //------------------------------------------------------------
            // Upstream completion (may happen multiple times)
            //------------------------------------------------------------
            "after_proxy" => {
                if let Some(f) = self.in_flight.get_mut(&request_id) {
                    f.status = e.status;
                }
            }

            //------------------------------------------------------------
            // Final response (authoritative end-of-request)
            //------------------------------------------------------------
            "response" => {
                if let Some(f) = self.in_flight.remove(&request_id) {
                    let latency_ms = now.duration_since(f.start).as_millis() as u64;

                    self.events.push_back(WindowEvent {
                        ts: now,
                        latency_ms: Some(latency_ms),
                        status: e.status.or(f.status),
                    });
                }
            }

            _ => {}
        }
    }

    fn evict(&mut self, now: Instant) {
        while let Some(ev) = self.events.front() {
            if now.duration_since(ev.ts) > self.window {
                self.events.pop_front();
            } else {
                break;
            }
        }
    }

    fn snapshot(&mut self) -> StatsSnapshot {
        let now = Instant::now();
        self.evict(now);

        let mut latency = Histogram::new(LATENCY_BUCKETS_MS);
        let mut status_2xx = 0;
        let mut status_4xx = 0;
        let mut status_5xx = 0;

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
        }

        StatsSnapshot {
            rps: self.events.len() as f64 / self.window.as_secs_f64(),
            window_events: self.events.len() as u64,
            latency: latency.snapshot(),
            status: (status_2xx, status_4xx, status_5xx),
        }
    }
}

struct StatsSnapshot {
    rps: f64,
    window_events: u64,
    latency: Vec<(String, u64)>,
    status: (u64, u64, u64), // 2xx, 4xx, 5xx
}

//-----------------------------------------------------------------------------
// Histograms
//-----------------------------------------------------------------------------

const LATENCY_BUCKETS_MS: &[u64] = &[1, 5, 10, 25, 50, 100, 250, 500, 1000];

#[derive(Clone)]
struct Histogram {
    buckets: &'static [u64],
    counts: Vec<u64>,
}

impl Histogram {
    fn new(buckets: &'static [u64]) -> Self {
        Self {
            buckets,
            counts: vec![0; buckets.len() + 1], // +∞ bucket
        }
    }

    fn record(&mut self, value: u64) {
        for (i, b) in self.buckets.iter().enumerate() {
            if value <= *b {
                self.counts[i] += 1;
                return;
            }
        }
        let last = self.counts.len() - 1;
        self.counts[last] += 1;
    }

    fn snapshot(&self) -> Vec<(String, u64)> {
        let mut out = Vec::new();

        for (i, c) in self.counts.iter().enumerate() {
            let label = if i == 0 {
                format!("0–{}ms", self.buckets[0])
            } else if i < self.buckets.len() {
                format!("{}–{}ms", self.buckets[i - 1] + 1, self.buckets[i])
            } else {
                format!(">{}ms", self.buckets.last().unwrap())
            };

            out.push((label, *c));
        }

        out
    }
}

fn systemtime_to_instant(ts: SystemTime) -> Instant {
    let now_sys = SystemTime::now();
    let now_inst = Instant::now();

    match ts.duration_since(now_sys) {
        Ok(delta) => now_inst + delta,
        Err(e) => now_inst - e.duration(),
    }
}
