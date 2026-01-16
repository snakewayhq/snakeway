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

use crate::logging::LogMode;
use anyhow::Result;
use serde_json::Value;
use std::collections::VecDeque;
use std::io::{self, BufRead};
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
    level: String,
    name: String,
    method: Option<String>,
    uri: Option<String>,
    status: Option<i64>,
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
        Some(LogEvent::Snakeway(SnakewayEvent {
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
            status: event.get("status").and_then(Value::as_i64),
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
    println!(
        "RPS: {:.1} | total: {} | 5xx: {}",
        snapshot.rps, snapshot.total, snapshot.errors
    );

    let total_latency: u64 = snapshot.latency.iter().map(|(_, c)| *c).sum();
    if total_latency > 0 {
        println!("Latency (window):");
        for (label, count) in &snapshot.latency {
            let pct = (*count as f64 / total_latency as f64) * 100.0;
            let bars = (pct / 5.0).round() as usize;
            println!("  {:<8} {:<20} {:>5.1}%", label, "█".repeat(bars), pct);
        }
    }

    let (ok, client, server) = snapshot.status;
    println!("Status: 2xx={} 4xx={} 5xx={}", ok, client, server);

    println!();
}

struct StatsAggregator {
    window: Duration,
    events: VecDeque<Instant>,

    // Basic stats
    total_seen: u64,
    errors: u64,

    // Histogram stats
    latency: Histogram,
    status_2xx: u64,
    status_4xx: u64,
    status_5xx: u64,
}

impl StatsAggregator {
    fn new(window: Duration) -> Self {
        Self {
            window,
            events: VecDeque::new(),
            total_seen: 0,
            errors: 0,
            latency: Histogram::new(LATENCY_BUCKETS_MS),
            status_2xx: 0,
            status_4xx: 0,
            status_5xx: 0,
        }
    }

    fn push(&mut self, event: &LogEvent) {
        let now = Instant::now();
        self.events.push_back(now);
        self.total_seen += 1;

        if let LogEvent::Snakeway(e) = event {
            if let Some(status) = e.status {
                match status {
                    200..=299 => self.status_2xx += 1,
                    400..=499 => self.status_4xx += 1,
                    500..=599 => {
                        self.status_5xx += 1;
                        self.errors += 1;
                    }
                    _ => {}
                }
            }

            // If there is latency, extract it.
            if let Some(ms) = extract_latency_ms(e) {
                self.latency.record(ms);
            }
        }

        self.evict(now);
    }

    fn evict(&mut self, now: Instant) {
        while let Some(ts) = self.events.front() {
            if now.duration_since(*ts) > self.window {
                self.events.pop_front();
            } else {
                break;
            }
        }
    }

    fn snapshot(&self) -> StatsSnapshot {
        StatsSnapshot {
            rps: self.events.len() as f64 / self.window.as_secs_f64(),
            total: self.total_seen,
            errors: self.errors,
            latency: self.latency.snapshot(),
            status: (self.status_2xx, self.status_4xx, self.status_5xx),
        }
    }
}

struct StatsSnapshot {
    rps: f64,
    total: u64,
    errors: u64,

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

    fn reset(&mut self) {
        self.counts.fill(0);
    }

    fn snapshot(&self) -> Vec<(String, u64)> {
        let mut out = Vec::new();

        for (i, c) in self.counts.iter().enumerate() {
            let label = if i < self.buckets.len() {
                format!("≤{}ms", self.buckets[i])
            } else {
                format!(">{}ms", self.buckets.last().unwrap())
            };
            out.push((label, *c));
        }

        out
    }
}

fn extract_latency_ms(_event: &SnakewayEvent) -> Option<u64> {
    None
}
